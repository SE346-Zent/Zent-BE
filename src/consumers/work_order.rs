use crate::core::config::AppConfig;
use crate::core::state::AppState;
use crate::entities::work_orders as work_orders_ent;
use crate::infrastructure::mq::{
    self,
    work_order::{setup_work_order_topology, WORK_ORDER_CREATED_QUEUE},
};
use futures::stream::StreamExt;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, BasicNackOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use sea_orm::EntityTrait;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tokio_executor_trait::Tokio as TokioExecutor;
use tracing::{error, info, warn};

pub async fn start_work_order_consumer(state: AppState) {
    let url = AppConfig::get().rabbitmq_url.clone();
    let db = state.db.clone();

    info!(
        message = "Work order consumer background task initialized",
        "Spawning independent connection lifecycle"
    );

    tokio::spawn(async move {
        loop {
            let fresh_url = mq::ensure_heartbeat(&url);
            let conn = match Connection::connect(
                &fresh_url,
                ConnectionProperties::default().with_executor(TokioExecutor::current()),
            )
            .await
            {
                Ok(c) => {
                    info!(
                        message = "RabbitMQ connection established for work order consumer",
                        "Activating communication transport"
                    );
                    Arc::new(c)
                }
                Err(e) => {
                    error!(
                        message = "RabbitMQ connection establishment failed for work order consumer",
                        error.message = %e,
                        error.details = ?e,
                        "Retrying connection fallback sequence in 5 seconds"
                    );
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            let channel = match conn.create_channel().await {
                Ok(c) => c,
                Err(e) => {
                    error!(
                        message = "AMQP channel creation failed for work order consumer",
                        error.message = %e,
                        error.details = ?e,
                        "Retrying channel recovery loop in 5 seconds"
                    );
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            if let Err(e) = setup_work_order_topology(&channel).await {
                error!(
                    message = "AMQP topology setup failed for work order queue",
                    error.message = %e,
                    error.details = ?e,
                    "Retrying topology registration in 5 seconds"
                );
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            let mut consumer = match channel
                .basic_consume(
                    WORK_ORDER_CREATED_QUEUE,
                    "",
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    error!(
                        message = "AMQP consumer binding failed for work order queue",
                        error.message = %e,
                        error.details = ?e,
                        queue = %WORK_ORDER_CREATED_QUEUE,
                        "Retrying queue consumption in 5 seconds"
                    );
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            info!(
                message = "Work order consumer stream activated",
                queue = %WORK_ORDER_CREATED_QUEUE,
                "Awaiting inbound message frames"
            );

            while let Some(delivery) = consumer.next().await {
                match delivery {
                    Ok(delivery) => {
                        if let Ok(payload_str) = std::str::from_utf8(&delivery.data) {
                            match serde_json::from_str::<serde_json::Value>(payload_str) {
                                Ok(payload) => {
                                    if let Some(id_str) = payload["id"].as_str() {
                                        if let Ok(id) = uuid::Uuid::parse_str(id_str) {
                                            if let Ok(Some(wo)) =
                                                work_orders_ent::Entity::find_by_id(id)
                                                    .one(db.as_ref())
                                                    .await
                                            {
                                                let pending_id = state
                                                    .lookup_tables
                                                    .work_order_statuses_by_name
                                                    .get("Pending")
                                                    .copied();

                                                if pending_id.map_or(true, |pid| {
                                                    wo.work_order_status_id != pid
                                                }) {
                                                    info!(
                                                        message = "Work order auto-assign skipped: assignment already fulfilled",
                                                        work_order_id = %id,
                                                        "Terminating duplicate transaction pathway"
                                                    );
                                                } else {
                                                    info!(
                                                        message = "Beginning automatic work order assignment routing",
                                                        work_order_id = %id,
                                                        "Evaluating assignment matching matrix"
                                                    );

                                                    let _success = crate::handlers::v1::work_orders::try_auto_assign_single(
                                                        &state,
                                                        db.clone(),
                                                        wo,
                                                    ).await;
                                                }
                                            } else {
                                                warn!(
                                                    message = "Work order processing skipped: record not found in database",
                                                    work_order_id = %id,
                                                    "Aborting auto-assignment logic"
                                                );
                                            }
                                        }
                                    }
                                    let _ = delivery.ack(BasicAckOptions::default()).await;
                                }
                                Err(e) => {
                                    error!(
                                        message = "Inbound payload serialization failed",
                                        error.message = %e,
                                        error.details = ?e,
                                        "Rejecting corrupted queue frame"
                                    );
                                    let _ = delivery
                                        .nack(BasicNackOptions {
                                            requeue: false,
                                            ..Default::default()
                                        })
                                        .await;
                                }
                            }
                        } else {
                            error!(
                                message = "Inbound queue frame contains invalid UTF-8 data",
                                "Rejecting unreadable message payload"
                            );
                            let _ = delivery
                                .nack(BasicNackOptions {
                                    requeue: false,
                                    ..Default::default()
                                })
                                .await;
                        }
                    }
                    Err(e) => {
                        error!(
                            message = "AMQP delivery stream connection severed",
                            error.message = %e,
                            error.details = ?e,
                            "Breaking trace stream consumer loop"
                        );
                        break;
                    }
                }
            }

            warn!(
                message = "Work order consumer transaction loop broken unexpectedly",
                "Initiating restart cooling delay for 5 seconds"
            );
            sleep(Duration::from_secs(5)).await;
        }
    });
}
