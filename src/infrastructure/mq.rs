use lapin::{
    Connection, ConnectionProperties,
};
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod email;

pub struct RabbitMQManager {
    url: String,
    connection: Mutex<Option<Arc<Connection>>>,
    is_stub: bool,
}

impl RabbitMQManager {
    pub fn new(url: &str) -> Arc<Self> {
        Arc::new(Self {
            url: url.to_string(),
            connection: Mutex::new(None),
            is_stub: false,
        })
    }

    pub fn is_stub(&self) -> bool {
        self.is_stub
    }

    pub async fn get_connection(&self) -> Result<Arc<Connection>, lapin::Error> {
        if self.is_stub {
            return Err(lapin::Error::ChannelsLimitReached); // Some error for stub
        }

        let mut guard = self.connection.lock().await;
        
        if let Some(conn) = &*guard {
            if conn.status().connected() {
                return Ok(conn.clone());
            }
        }

        let conn = Connection::connect(&self.url, ConnectionProperties::default()).await?;
        let arc_conn = Arc::new(conn);
        
        *guard = Some(arc_conn.clone());
        Ok(arc_conn)
    }

    /// Create a stub manager that will fail to connect
    pub fn stub() -> Arc<Self> {
        Arc::new(Self {
            url: "amqp://invalid:5672".to_string(),
            connection: Mutex::new(None),
            is_stub: true,
        })
    }
}

pub async fn init_rabbitmq(url: &str) -> Arc<RabbitMQManager> {
    RabbitMQManager::new(url)
}


