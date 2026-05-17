use std::sync::Arc;
use lapin::Connection;
use crate::infrastructure::mq::email::EmailProducer;
use crate::core::errors::AppError;
use serde_json::Value;

/// Helper to publish an email payload to RabbitMQ with standardized error handling.
pub async fn publish_email_task(
    rabbitmq_connection: &Arc<Connection>,
    payload: Value,
    task_name: &str,
) -> Result<(), AppError> {
    let producer = EmailProducer::new(Some(rabbitmq_connection.clone()));
    
    producer.publish(payload.to_string().as_bytes()).await
        .map_err(|e| {
            tracing::error!("Failed to enqueue {} task into RabbitMQ: {}", task_name, e);
            AppError::Internal(anyhow::anyhow!("Failed to send {}", task_name))
        })?;

    Ok(())
}
