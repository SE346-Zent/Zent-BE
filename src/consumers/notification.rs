use futures::stream::StreamExt;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, BasicNackOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tokio_executor_trait::Tokio as TokioExecutor;
use tracing::{error, info};
use uuid::Uuid;

use crate::core::config::AppConfig;
use crate::core::state::AppState;
use crate::infrastructure::mq::fcm::FcmProducer;
use crate::infrastructure::mq::notification::{setup_notification_topology, NOTIFICATION_QUEUE};

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
/// Consumes from `notification_queue`, fetches the user's FCM token,
/// and publishes an FCM push message for real-time delivery.
///
/// MongoDB save and Valkey unread counter increment are now handled
/// in `send_notification()` before the outbox is created.
pub async fn start_notification_consumer(state: AppState) {
    let url = AppConfig::get().rabbitmq_url.clone();

    tokio::spawn(async move {
        loop {
            let fresh_url = crate::infrastructure::mq::ensure_heartbeat(&url);
            let conn = match Connection::connect(
                &fresh_url,
                ConnectionProperties::default().with_executor(TokioExecutor::current()),
            )
            .await
            {
                Ok(c) => Arc::new(c),
                Err(e) => {
                    error!(
                        message = "RabbitMQ connection establishment failed",
                        error.message = %e,
                        error.details = ?e,
                        "Retrying connection in 5 seconds"
                    );
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            let channel = match conn.create_channel().await {
                Ok(c) => c,
                Err(e) => {
                    error!(
                        message = "RabbitMQ channel creation failed",
                        error.message = %e,
                        error.details = ?e,
                        "Reconnecting transport"
                    );
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            if let Err(e) = setup_notification_topology(&channel).await {
                error!(
                    message = "AMQP topology setup failed",
                    error.message = %e,
                    error.details = ?e,
                    "Retrying topology setup in 5 seconds"
                );
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            let mut consumer = match channel
                .basic_consume(
                    NOTIFICATION_QUEUE,
                    "",
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    error!(
                        message = "AMQP queue consumer attachment failed",
                        error.message = %e,
                        error.details = ?e,
                        queue = %NOTIFICATION_QUEUE,
                        "Retrying consumption in 5 seconds"
                    );
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            info!(
                message = "Notification consumer transaction loop started",
                queue = %NOTIFICATION_QUEUE,
                "Awaiting queue deliveries"
            );

            while let Some(delivery) = consumer.next().await {
                match delivery {
                    Ok(delivery) => {
                        if let Ok(payload_str) = std::str::from_utf8(&delivery.data) {
                            match serde_json::from_str::<NotificationMessage>(payload_str) {
                                Ok(msg) => {
                                    let success = handle_notification_message(&state, &msg).await;

                                    if success {
                                        let _ = delivery.ack(BasicAckOptions::default()).await;
                                    } else {
                                        // Specific tracking properties isolated from message string
                                        error!(
                                            message = "Notification processing transaction rejected",
                                            notification_id = %msg.notification_id,
                                            user_id = %msg.user_id,
                                            "Sending negative acknowledgment (NACK) without requeue"
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
                                message = "Inbound queue delivery contains invalid UTF-8 data",
                                "Rejecting unreadable queue frame"
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
        }
    });
}

/// Process a single notification message (FCM push only).
///
/// MongoDB save and Valkey unread counter increment now happen
/// in `send_notification()` before the outbox is created.
async fn handle_notification_message(state: &AppState, msg: &NotificationMessage) -> bool {
    if let Err(e) = send_fcm_push(state, msg).await {
        // Unified terminal processing failure log containing contextual fields
        error!(
            message = "FCM delivery lifecycle failed",
            error.message = %e,
            error.details = ?e,
            notification_id = %msg.notification_id,
            user_id = %msg.user_id,
            "Halting execution pathway"
        );
        return false;
    }
    true
}

/// Send FCM push notification.
async fn send_fcm_push(state: &AppState, msg: &NotificationMessage) -> Result<(), anyhow::Error> {
    use crate::entities::users;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let user = users::Entity::find()
        .filter(users::Column::Id.eq(msg.user_id))
        .one(state.db.as_ref())
        .await?;

    let fcm_token = match user.and_then(|u| u.fcm_token) {
        Some(t) => t,
        // Empty tokens represent valid user settings configurations; bubble up clean exit status
        None => return Ok(()),
    };

    let data_value: serde_json::Value =
        serde_json::from_str(&msg.data).unwrap_or(serde_json::Value::Null);

    let payload = serde_json::json!({
        "notificationId": msg.notification_id,
        "userId": msg.user_id,
        "fcmToken": fcm_token,
        "title": msg.title,
        "body": msg.body,
        "data": data_value,
    });

    let producer = FcmProducer::new(state.rabbitmq.clone());

    producer.publish(&serde_json::to_vec(&payload)?).await?;

    Ok(())
}
