use lapin::{
    Connection, ConnectionProperties,
};

pub mod email;

/// Initialize RabbitMQ: connect and return connection.
pub async fn init_rabbitmq(url: &str) -> Result<Connection, lapin::Error> {
    Connection::connect(url, ConnectionProperties::default()).await
}
