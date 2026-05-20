use std::sync::Arc;
use std::time::Duration;
use lapin::{
    options::{BasicConsumeOptions, BasicAckOptions, BasicNackOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use tokio_executor_trait::Tokio as TokioExecutor;
use futures::stream::StreamExt;
use tracing::{info, error, warn};
use tokio::time::sleep;
use crate::core::config::AppConfig;
use crate::core::state::AppState;
use crate::infrastructure::mq::{self, work_order::{WORK_ORDER_CREATED_QUEUE, setup_work_order_topology}};
use crate::entities::work_orders as work_orders_ent;
use sea_orm::EntityTrait;

pub async fn start_work_order_consumer(state: AppState) {
    let url = AppConfig::get().rabbitmq_url.clone();
    let db = state.db.clone();

    // The consumer always spawns its own independent connection loop.
    // It does NOT depend on state.rabbitmq — that shared connection may be None
    // (failed at startup) or may have died, yet the producer can still publish
    // successfully by opening fresh connections. Mirroring that approach here
    // ensures the consumer is always alive regardless of the shared handle.
    info!("Work order consumer task spawned — will dial RabbitMQ independently");

    tokio::spawn(async move {
        loop {
            // --- Establish a dedicated connection for this consumer ---
            let fresh_url = mq::ensure_heartbeat(&url);
            let conn = match Connection::connect(&fresh_url, ConnectionProperties::default().with_executor(TokioExecutor::current())).await {
                Ok(c) => {
                    info!("Work order consumer connected to RabbitMQ");
                    Arc::new(c)
                }
                Err(e) => {
                    error!("Work order consumer: failed to connect to RabbitMQ: {:?}. Retrying in 5s...", e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            // --- Open a channel ---
            let channel = match conn.create_channel().await {
                Ok(c) => c,
                Err(e) => {
                    error!("Work order consumer: failed to create channel: {:?}. Reconnecting...", e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            // --- Declare topology (idempotent) ---
            if let Err(e) = setup_work_order_topology(&channel).await {
                error!("Work order consumer: failed to setup topology: {:?}. Retrying in 5s...", e);
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            // --- Attach consumer
            // Use an empty tag so the broker generates a unique one per session.
            // A fixed tag causes PRECONDITION_FAILED when the broker still has the
            // previous connection's consumer registered (e.g. after a fast restart),
            // silently preventing any message delivery.
            let mut consumer = match channel.basic_consume(
                WORK_ORDER_CREATED_QUEUE,
                "",   // broker-generated unique tag
                BasicConsumeOptions::default(),
                FieldTable::default(),
            ).await {
                Ok(c) => c,
                Err(e) => {
                    error!("Work order consumer: failed to attach to queue: {:?}. Retrying in 5s...", e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            info!("Work order consumer listening on '{}'", WORK_ORDER_CREATED_QUEUE);

            while let Some(delivery) = consumer.next().await {
                match delivery {
                    Ok(delivery) => {
                        if let Ok(payload_str) = std::str::from_utf8(&delivery.data) {
                            match serde_json::from_str::<serde_json::Value>(payload_str) {
                                Ok(payload) => {
                                    if let Some(id_str) = payload["id"].as_str() {
                                        if let Ok(id) = uuid::Uuid::parse_str(id_str) {
                                            if let Ok(Some(wo)) = work_orders_ent::Entity::find_by_id(id).one(db.as_ref()).await {
                                                // Guard: skip if the WO is already assigned (direct path in
                                                // create handler ran first — no double-assign needed)
                                                let pending_id = state.lookup_tables
                                                    .work_order_statuses_by_name
                                                    .get("Pending")
                                                    .copied();
                                                if pending_id.map_or(true, |pid| wo.work_order_status_id != pid) {
                                                    info!("WO {} is already assigned — skipping MQ auto-assign", id);
                                                } else {
                                                    info!("MQ: Processing auto-assign for WO {}", id);
                                                    let success = crate::handlers::v1::work_orders::try_auto_assign_single(
                                                        &state,
                                                        db.clone(),
                                                        wo,
                                                    ).await;
                                                    if !success {
                                                        warn!("MQ auto-assign did not complete for WO {}", id);
                                                    }
                                                }
                                            } else {
                                                warn!("WO {} not found in DB — skipping auto-assign", id);
                                            }
                                        }
                                    }
                                    let _ = delivery.ack(BasicAckOptions::default()).await;
                                }
                                Err(e) => {
                                    error!("Work order consumer: failed to parse payload: {:?}", e);
                                    let _ = delivery.nack(BasicNackOptions { requeue: false, ..Default::default() }).await;
                                }
                            }
                        } else {
                            let _ = delivery.nack(BasicNackOptions { requeue: false, ..Default::default() }).await;
                        }
                    }
                    Err(e) => {
                        error!("Work order consumer: delivery stream error: {:?}", e);
                        break; // Drop out of the inner loop → reconnect
                    }
                }
            }

            warn!("Work order consumer: delivery stream ended — reconnecting in 5s...");
            sleep(Duration::from_secs(5)).await;
        }
    });
}
