use std::sync::Arc;
use std::time::Duration;
use lapin::{
    options::{BasicConsumeOptions, BasicAckOptions, BasicNackOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use futures::stream::StreamExt;
use tracing::{info, error, warn};
use tokio::time::{sleep};
use crate::core::config::AppConfig;
use crate::core::state::AppState;
use crate::infrastructure::mq::work_order::{WORK_ORDER_CREATED_QUEUE, setup_work_order_topology};
use crate::entities::work_orders as work_orders_ent;
use sea_orm::EntityTrait;

pub async fn start_work_order_consumer(state: AppState) {
    let mut conn_opt = match state.rabbitmq {
        Some(ref c) => c.clone(),
        None => return,
    };

    let url = AppConfig::get().rabbitmq_url.clone();
    let db = state.db.clone();

    tokio::spawn(async move {
        loop {
            let channel = match conn_opt.create_channel().await {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to create MQ channel for work order consumer: {:?}. Reconnecting...", e);
                    let fresh_url = if url.contains("heartbeat=") {
                            url.clone()
                        } else if url.contains('?') {
                            format!("{}&heartbeat=60", url)
                        } else {
                            format!("{}?heartbeat=60", url)
                        };
                    match Connection::connect(
                        &fresh_url,
                        ConnectionProperties::default(),
                    ).await {
                        Ok(new_conn) => {
                            info!("Work order consumer established new RabbitMQ connection");
                            conn_opt = Arc::new(new_conn);
                            continue;
                        }
                        Err(re) => {
                            error!("Failed to reconnect work order consumer: {:?}. Retrying in 5s...", re);
                            sleep(Duration::from_secs(5)).await;
                            continue;
                        }
                    }
                }
            };

            if let Err(e) = setup_work_order_topology(&channel).await {
                error!("Failed to setup work order topology: {:?}. Retrying in 5s...", e);
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            let mut consumer = match channel.basic_consume(
                WORK_ORDER_CREATED_QUEUE,
                "work_order_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            ).await {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to attach consumer to work_order_created_queue: {:?}. Retrying in 5s...", e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            info!("Work Order Consumer listening!");

            while let Some(delivery) = consumer.next().await {
                match delivery {
                    Ok(delivery) => {
                        if let Ok(payload_str) = std::str::from_utf8(&delivery.data) {
                            match serde_json::from_str::<serde_json::Value>(payload_str) {
                                Ok(payload) => {
                                    if let Some(id_str) = payload["id"].as_str() {
                                        if let Ok(id) = uuid::Uuid::parse_str(id_str) {
                                            info!("Processing auto-assign for WO {}", id);
                                            if let Ok(Some(wo)) = work_orders_ent::Entity::find_by_id(id).one(db.as_ref()).await {
                                                let _ = crate::handlers::v1::work_orders::try_auto_assign_single(
                                                    &state,
                                                    db.clone(),
                                                    wo,
                                                ).await;
                                            }
                                        }
                                    }
                                    let _ = delivery.ack(BasicAckOptions::default()).await;
                                }
                                Err(e) => {
                                    error!("Failed to parse WO payload: {:?}", e);
                                    let _ = delivery.nack(BasicNackOptions { requeue: false, ..Default::default() }).await;
                                }
                            }
                        } else {
                            let _ = delivery.nack(BasicNackOptions { requeue: false, ..Default::default() }).await;
                        }
                    }
                    Err(error) => {
                        error!("MQ delivery stream error: {:?}", error);
                        break;
                    }
                }
            }
            warn!("Work order consumer loop exited, reconnecting in 5s...");
            sleep(Duration::from_secs(5)).await;
        }
    });
}
