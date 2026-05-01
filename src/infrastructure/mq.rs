use lapin::{
    Connection, ConnectionProperties,
};
use std::sync::Arc;

pub mod email;

/// Lightweight RabbitMQ client.
/// Unlike SeaORM and redis, lapin's `Connection` is already a handle that can
/// be cloned cheaply. We initialize it once at startup.
pub struct RabbitMQClient {
    connection: Option<Arc<Connection>>,
}

impl RabbitMQClient {
    pub fn is_stub(&self) -> bool {
        self.connection.is_none()
    }

    pub async fn get_connection(&self) -> Result<Arc<Connection>, lapin::Error> {
        self.connection.clone().ok_or_else(|| {
            lapin::Error::ChannelsLimitReached // Or another suitable error for stub mode
        })
    }

    /// Wrap an existing connection.
    pub fn from_connection(conn: Connection) -> Arc<Self> {
        Arc::new(Self {
            connection: Some(Arc::new(conn)),
        })
    }

    /// Create a non-functional stub for tests that don't need MQ access.
    pub fn stub() -> Arc<Self> {
        Arc::new(Self {
            connection: None,
        })
    }
}

/// Backward-compatible alias so existing tests that reference
/// `RabbitMQManager` continue to compile.
pub type RabbitMQManager = RabbitMQClient;

/// Initialize RabbitMQ: connect and return client.
pub async fn init_rabbitmq(url: &str) -> Result<Arc<RabbitMQClient>, lapin::Error> {
    let conn = Connection::connect(url, ConnectionProperties::default()).await?;
    Ok(Arc::new(RabbitMQClient {
        connection: Some(Arc::new(conn)),
    }))
}
