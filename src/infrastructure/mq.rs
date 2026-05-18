use lapin::{
    Connection, ConnectionProperties,
};
use tokio_executor_trait::Tokio as TokioExecutor;

pub mod email;
pub mod work_order;
pub mod fcm;
pub mod notification;

/// Append `heartbeat=60` to the AMQP URL if not already present.
/// Lapin 2.x reads heartbeat from the URI query parameter.
pub(crate) fn ensure_heartbeat(url: &str) -> String {
    let heartbeat_param = "heartbeat=60";
    if url.contains("heartbeat=") {
        url.to_string()
    } else if url.contains('?') {
        format!("{}&{}", url, heartbeat_param)
    } else {
        format!("{}?{}", url, heartbeat_param)
    }
}

/// Initialize RabbitMQ: connect and return connection.
/// Heartbeat (60s) is injected into the URL to detect silent TCP drops
/// (NAT, firewall, etc.) even when no application traffic is flowing.
pub async fn init_rabbitmq(url: &str) -> Result<Connection, lapin::Error> {
    Connection::connect(&ensure_heartbeat(url), ConnectionProperties::default().with_executor(TokioExecutor::current())).await
}
