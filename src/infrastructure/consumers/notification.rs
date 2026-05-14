use std::sync::Arc;
use std::time::Duration;
use lapin::{
    options::{BasicConsumeOptions, BasicAckOptions, BasicNackOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::{info, error};
use tokio::time::sleep;
use uuid::Uuid;

use crate::core::config::AppConfig;
use crate::core::state::AppState;
use crate::infrastructure::mq::notification::{NOTIFICATION_QUEUE, setup_notification_topology};
use crate::infrastructure::mq::fcm::FcmProducer;

/// Payload received from the notification queue (relayed by the outbox cron job).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationMessage {
    pub notification_id: Uuid,
    pub user_id: Uuid,
    pub category_id: i32,
    pub title: String,
    pub body: String,
    pub data: String,
}

/// Start the notification consumer background task.
///
/// Phase 2 of the outbox pattern:
/// 1. Consumes from `notification_queue`
/// 2. Saves the notification document into MongoDB (bucket pattern)
/// 3. Increments the Valkey unread counter for the user
/// 4. Publishes an FCM push message for real-time delivery
pub async fn start_notification_consumer(state: AppState) {
    let connection = match &state.rabbitmq {
        Some(c) => c.clone(),
        None => return,
    };

    let url = AppConfig::get().rabbitmq_url.clone();

    tokio::spawn(async move {
        let mut conn_opt = connection;

        loop {
            let channel = match conn_opt.create_channel().await {
                Ok(c) => c,
                Err(e) => {
                    error!("Notification consumer: Failed to create channel: {:?}. Reconnecting...", e);
                    match Connection::connect(&url, ConnectionProperties::default()).await {
                        Ok(new_conn) => {
                            conn_opt = Arc::new(new_conn);
                            continue;
                        }
                        Err(re) => {
                            error!("Notification consumer: Failed to reconnect: {:?}. Retrying in 5s...", re);
                            sleep(Duration::from_secs(5)).await;
                            continue;
                        }
                    }
                }
            };

            if let Err(e) = setup_notification_topology(&channel).await {
                error!("Notification consumer: Failed to setup topology: {:?}. Retrying in 5s...", e);
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            let mut consumer = match channel.basic_consume(
                NOTIFICATION_QUEUE,
                "notification_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            ).await {
                Ok(c) => c,
                Err(e) => {
                    error!("Notification consumer: Failed to attach: {:?}. Retrying in 5s...", e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            info!("Notification consumer listening on {}", NOTIFICATION_QUEUE);

            while let Some(delivery) = consumer.next().await {
                match delivery {
                    Ok(delivery) => {
                        if let Ok(payload_str) = std::str::from_utf8(&delivery.data) {
                            match serde_json::from_str::<NotificationMessage>(payload_str) {
                                Ok(msg) => {
                                    let success = handle_notification_message(
                                        &state, &msg,
                                    ).await;

                                    if success {
                                        let _ = delivery.ack(BasicAckOptions::default()).await;
                                    } else {
                                        error!("Notification consumer: Processing failed for notif {}", msg.notification_id);
                                        let _ = delivery.nack(BasicNackOptions {
                                            requeue: false,
                                            ..Default::default()
                                        }).await;
                                    }
                                }
                                Err(e) => {
                                    error!("Notification consumer: Invalid message: {:?}", e);
                                    let _ = delivery.nack(BasicNackOptions {
                                        requeue: false,
                                        ..Default::default()
                                    }).await;
                                }
                            }
                        } else {
                            let _ = delivery.nack(BasicNackOptions {
                                requeue: false,
                                ..Default::default()
                            }).await;
                        }
                    }
                    Err(e) => {
                        error!("Notification consumer: Stream error: {:?}", e);
                        break;
                    }
                }
            }
        }
    });
}

/// Process a single notification message (FCM push only).
///
/// MongoDB save and Valkey unread counter increment now happen
/// in `send_notification()` before the outbox is created.
async fn handle_notification_message(
    state: &AppState,
    msg: &NotificationMessage,
) -> bool {
    // Send FCM push notification
    if let Err(e) = send_fcm_push(state, msg).await {
        error!("Failed to send FCM push for notification {}: {:?}", msg.notification_id, e);
        return false;
    }

    true
}



/// Send FCM push notification.
async fn send_fcm_push(
    state: &AppState,
    msg: &NotificationMessage,
) -> Result<(), anyhow::Error> {
    // Fetch the user's FCM token
    use sea_orm::{ColumnTrait, QueryFilter, EntityTrait};
    use crate::entities::users;

    let user = users::Entity::find()
        .filter(users::Column::Id.eq(msg.user_id))
        .one(state.db.as_ref())
        .await?;

    let fcm_token = match user.and_then(|u| u.fcm_token) {
        Some(t) => t,
        None => return Ok(()), // No FCM token — skip push
    };

    // Parse data for the FCM payload
    let data_value: serde_json::Value = serde_json::from_str(&msg.data).unwrap_or(serde_json::Value::Null);

    let payload = serde_json::json!({
        "notificationId": msg.notification_id,
        "userId": msg.user_id,
        "fcmToken": fcm_token,
        "title": msg.title,
        "body": msg.body,
        "data": data_value,
    });

    let producer = FcmProducer::new(state.rabbitmq.clone());
    producer
        .publish(&serde_json::to_vec(&payload)?)
        .await
        .map_err(|e| {
            error!("Failed to publish FCM message: {}", e);
            e
        })?;

    Ok(())
}
