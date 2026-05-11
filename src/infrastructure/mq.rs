use lapin::{
    Connection, ConnectionProperties,
};

pub mod email;
pub mod work_order;

/// Initialize RabbitMQ: connect and return connection.
pub async fn init_rabbitmq(url: &str) -> Result<Connection, lapin::Error> {
    Connection::connect(url, ConnectionProperties::default()).await
}
