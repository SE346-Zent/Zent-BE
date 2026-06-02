use crate::core::errors::AppError;
use crate::infrastructure::mq::email::EmailProducer;
use lapin::Connection;
use serde_json::Value;
use std::sync::Arc;

/// Helper to publish an email payload to RabbitMQ with standardized error handling.
pub async fn publish_email_task(
    rabbitmq_connection: &Arc<Connection>,
    payload: Value,
    task_name: &str,
) -> Result<(), AppError> {
    let producer = EmailProducer::new(Some(rabbitmq_connection.clone()));

    producer
        .publish(payload.to_string().as_bytes())
        .await
        .map_err(|e| {
            tracing::error!(
                message = "AMQP task publication failed",
                error.message = %e,
                error.details = ?e,
                task_type = %task_name,
                "Failed to enqueue background message frame into queue architecture"
            );

            AppError::Internal(e.context(format!(
                "Failed to enqueue task payload for context: {}",
                task_name
            )))
        })?;

    Ok(())
}
