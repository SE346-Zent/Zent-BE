use lapin::{
    Connection, ConnectionProperties,
};
use tokio_executor_trait::Tokio as TokioExecutor;

pub mod email;
pub mod work_order;
pub mod fcm;
pub mod notification;

/// Ensure the RabbitMQ connection URL contains the `heartbeat=60` query parameter.
///
/// # Arguments
/// * `amqp_url` - The original RabbitMQ connection URL.
///
/// # Returns
/// A string with the heartbeat parameter appended or maintained.
pub(crate) fn ensure_heartbeat(amqp_url: &str) -> String {
    let heartbeat_param = "heartbeat=60";
    if amqp_url.contains("heartbeat=") {
        amqp_url.to_string()
    } else if amqp_url.contains('?') {
        format!("{}&{}", amqp_url, heartbeat_param)
    } else {
        format!("{}?{}", amqp_url, heartbeat_param)
    }
}

/// Initialize the RabbitMQ connection using the provided URL.
///
/// # Arguments
/// * `amqp_url` - The RabbitMQ connection URL.
///
/// # Returns
/// A result containing the `lapin::Connection` or a `lapin::Error`.
pub async fn init_rabbitmq(amqp_url: &str) -> Result<Connection, lapin::Error> {
    Connection::connect(&ensure_heartbeat(amqp_url), ConnectionProperties::default()).await
}
