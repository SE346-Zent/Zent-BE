use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, ActiveModelTrait};
use std::sync::Arc;
use lapin::Connection;
use tokio_cron_scheduler::Job;
use tracing::{info, error, warn};

use crate::entities::outbox_records;
use crate::infrastructure::mq::notification::NotificationProducer;

/// Cron job that relays undelivered outbox records to the notification message queue.
///
/// Runs every 10 seconds. For each undelivered outbox entry:
/// 1. Serializes the payload (notification_id, user_id, category_id, title, body, data)
/// 2. Publishes to the notification exchange
/// 3. On ACK, marks the outbox row as delivered = true
///
/// This replaces Debezium/CDC — a simple poll-based message relay.
pub fn relay_outbox_job(
    db: sea_orm::DatabaseConnection,
    rabbitmq: Option<Arc<Connection>>,
) -> Result<Job, anyhow::Error> {
    // Run every 10 seconds: "0/10 * * * * *"
    let job = Job::new_async("0/10 * * * * *", move |_uuid, _l| {
        let db_clone = db.clone();
        let rmq = rabbitmq.clone();
        Box::pin(async move {
            info!("Running outbox relay job...");

            // Fetch all undelivered outbox entries (capped at 100 per run)
            let entries = match outbox_records::Entity::find()
                .filter(outbox_records::Column::Delivered.eq(false))
                .limit(100)
                .all(&db_clone)
                .await
            {
                Ok(rows) => rows,
                Err(e) => {
                    error!("Outbox relay: Failed to fetch undelivered records: {:?}", e);
                    return;
                }
            };

            if entries.is_empty() {
                return;
            }

            let producer = NotificationProducer::new(rmq);

            for entry in &entries {
                // Build the notification payload for the consumer
                let payload = serde_json::json!({
                    "notificationId": entry.notification_id,
                    "userId": entry.user_id,
                    "categoryId": entry.category_id,
                    "title": entry.title,
                    "body": entry.body,
                    "data": entry.data,
                });

                let payload_bytes = match serde_json::to_vec(&payload) {
                    Ok(b) => b,
                    Err(e) => {
                        error!("Outbox relay: Failed to serialize payload for {}: {:?}", entry.outbox_id, e);
                        continue;
                    }
                };

                // Publish to MQ — waits for ACK
                match producer.publish(&payload_bytes).await {
                    Ok(()) => {
                        // Mark as delivered on successful publish + ACK
                        let mut active: outbox_records::ActiveModel = entry.clone().into();
                        active.delivered = Set(true);
                        if let Err(e) = active.update(&db_clone).await {
                            error!("Outbox relay: Failed to mark {} as delivered: {:?}", entry.outbox_id, e);
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Outbox relay: Failed to publish outbox {} to MQ: {:?}. Will retry next cycle.",
                            entry.outbox_id, e
                        );
                        // Don't mark delivered — retry on next cron tick
                    }
                }
            }

            info!("Outbox relay: processed {} entries", entries.len());
        })
    })?;

    Ok(job)
}
