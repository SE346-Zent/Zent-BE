use lapin::{
    options::{ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions, BasicPublishOptions},
    types::FieldTable,
    BasicProperties, Connection, ConnectionProperties, ExchangeKind,
};
use std::sync::Arc;
use tokio::sync::Mutex;

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
        
        Self::initialize_structures(&arc_conn).await?;
        
        *guard = Some(arc_conn.clone());
        Ok(arc_conn)
    }

    async fn initialize_structures(conn: &Connection) -> Result<(), lapin::Error> {
        let channel = conn.create_channel().await?;

        // 1. Declare Dead Letter Exchange and Queue
        channel.exchange_declare(
            "email_dlx",
            ExchangeKind::Direct,
            ExchangeDeclareOptions { durable: true, ..Default::default() },
            FieldTable::default(),
        ).await?;

        channel.queue_declare(
            "email_dlq",
            QueueDeclareOptions { durable: true, ..Default::default() },
            FieldTable::default(),
        ).await?;

        channel.queue_bind(
            "email_dlq",
            "email_dlx",
            "send_email",
            QueueBindOptions::default(),
            FieldTable::default(),
        ).await?;

        // 2. Declare Main Exchange and Queue attaching DLX fallbacks
        channel.exchange_declare(
            "email_exchange",
            ExchangeKind::Direct,
            ExchangeDeclareOptions { durable: true, ..Default::default() },
            FieldTable::default(),
        ).await?;

        let mut queue_args = FieldTable::default();
        queue_args.insert(
            "x-dead-letter-exchange".into(),
            lapin::types::AMQPValue::LongString("email_dlx".into()),
        );

        channel.queue_declare(
            "email_queue",
            QueueDeclareOptions { durable: true, ..Default::default() },
            queue_args,
        ).await?;

        channel.queue_bind(
            "email_queue",
            "email_exchange",
            "send_email",
            QueueBindOptions::default(),
            FieldTable::default(),
        ).await?;

        Ok(())
    }

    pub async fn publish_email_message(&self, payload: &[u8]) -> Result<(), anyhow::Error> {
        if self.is_stub {
            return Ok(()); // Success for stub
        }
        let conn = self.get_connection().await?;
        let channel = conn.create_channel().await?;
        channel.basic_publish(
            "email_exchange",
            "send_email",
            BasicPublishOptions::default(),
            payload,
            BasicProperties::default().with_delivery_mode(2), // Persistent
        ).await?;
        Ok(())
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


