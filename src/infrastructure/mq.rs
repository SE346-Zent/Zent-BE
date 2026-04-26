use lapin::{
    options::{ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions, BasicPublishOptions, BasicConsumeOptions, BasicAckOptions, BasicNackOptions},
    types::FieldTable,
    BasicProperties, Connection, ConnectionProperties, ExchangeKind,
};
use std::sync::Arc;
use futures::stream::StreamExt;
use tracing::{info, error, warn};
use lettre::{Message, SmtpTransport, Transport};
use lettre::transport::smtp::authentication::Credentials;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

use crate::core::config::AppConfig;

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

pub async fn start_email_consumer(manager: Arc<RabbitMQManager>) {
    tokio::spawn(async move {
        loop {
            let conn = match manager.get_connection().await {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to get RabbitMQ connection for consumer: {:?}. Retrying in 5s...", e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            let channel = match conn.create_channel().await {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to create MQ channel for consumer: {:?}. Retrying in 5s...", e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            let mut consumer = match channel.basic_consume(
                "email_queue",
                "email_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            ).await {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to attach consumer to email_queue: {:?}. Retrying in 5s...", e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            info!("Background Lettre Consumer listening to email_queue natively!");

            while let Some(delivery) = consumer.next().await {
                match delivery {
                    Ok(delivery) => {
                        if let Ok(payload) = std::str::from_utf8(&delivery.data) {
                            info!("Received new email task from main queue. Payload: {}", payload);
                            
                            let success = send_email_with_lettre(payload).await;

                            if success {
                                let _ = delivery.ack(BasicAckOptions::default()).await;
                            } else {
                                error!("Failed to send email. Bouncing to DLQ!");
                                let _ = delivery.nack(BasicNackOptions {
                                    requeue: false,
                                    ..Default::default()
                                }).await;
                            }
                        } else {
                            let _ = delivery.nack(BasicNackOptions {
                                requeue: false,
                                ..Default::default()
                            }).await;
                        }
                    }
                    Err(error) => {
                        error!("Error within RabbitMQ consumer delivery stream: {:?}", error);
                        break; // Break inner loop to trigger reconnection
                    }
                }
            }
            
            warn!("Consumer loop exited, attempting to reconnect in 5s...");
            sleep(Duration::from_secs(5)).await;
        }
    });
}

// Lettre execution resolving logic
async fn send_email_with_lettre(payload: &str) -> bool {
    let parsed: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("Failed to parse email payload JSON: {:?}", e);
            return false;
        }
    };

    let to = match parsed["to"].as_str() {
        Some(v) => v,
        None => {
            tracing::error!("Missing 'to' field in email payload");
            return false;
        }
    };
    let subject = parsed["subject"].as_str().unwrap_or("System Notification");
    let body = parsed["body"].as_str().unwrap_or("");

    let cfg = AppConfig::get();
    let email = match Message::builder()
        .from(format!("Zent System <{}>", cfg.smtp_username).parse().unwrap())
        .to(match to.parse() {
            Ok(addr) => addr,
            Err(e) => {
                tracing::error!("Invalid recipient email '{}': {:?}", to, e);
                return false;
            }
        })
        .subject(subject)
        .singlepart(lettre::message::SinglePart::html(String::from(body)))
    {
        Ok(msg) => msg,
        Err(e) => {
            tracing::error!("Failed to build email message: {:?}", e);
            return false;
        }
    };

    let creds = Credentials::new(cfg.smtp_username.clone(), cfg.smtp_password.clone());

    let mailer = SmtpTransport::relay("smtp.gmail.com")
        .unwrap()
        .credentials(creds)
        .build();

    match mailer.send(&email) {
        Ok(_) => {
            tracing::info!("Email sent successfully to {}", to);
            true
        }
        Err(e) => {
            tracing::error!("SMTP delivery failed to {}: {:?}", to, e);
            false 
        }
    }
}
