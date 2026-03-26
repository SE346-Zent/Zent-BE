use lapin::{
    options::{ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions, BasicPublishOptions, BasicConsumeOptions, BasicAckOptions, BasicNackOptions},
    types::FieldTable,
    BasicProperties, Connection, ConnectionProperties, ExchangeKind,
};
use std::sync::Arc;
use futures::stream::StreamExt;
use tracing::{info, error};
use lettre::{Message, SmtpTransport, Transport};
use lettre::transport::smtp::authentication::Credentials;

use crate::config::AppConfig;

pub async fn init_rabbitmq(url: &str) -> Result<Arc<Connection>, lapin::Error> {
    let connection = Connection::connect(url, ConnectionProperties::default()).await?;
    let channel = connection.create_channel().await?;

    // 1. Declare Dead Letter Exchange and Queue dynamically mapping failures
    channel
        .exchange_declare(
            "email_dlx",
            ExchangeKind::Direct,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_declare(
            "email_dlq",
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_bind(
            "email_dlq",
            "email_dlx",
            "send_email",
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    // 2. Declare Main Exchange and Queue attaching DLX fallbacks natively
    channel
        .exchange_declare(
            "email_exchange",
            ExchangeKind::Direct,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    let mut queue_args = FieldTable::default();
    queue_args.insert(
        "x-dead-letter-exchange".into(),
        lapin::types::AMQPValue::LongString("email_dlx".into()),
    );

    channel
        .queue_declare(
            "email_queue",
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            queue_args,
        )
        .await?;

    channel
        .queue_bind(
            "email_queue",
            "email_exchange",
            "send_email",
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    info!("RabbitMQ structures initialized with Dead Letter Queue Fallbacks strictly bound!");
    Ok(Arc::new(connection))
}

pub async fn publish_email_message(
    conn: &lapin::Connection,
    payload: &[u8],
) -> Result<(), anyhow::Error> {
    let channel = conn.create_channel().await?;
    channel
        .basic_publish(
            "email_exchange",
            "send_email",
            BasicPublishOptions::default(),
            payload,
            BasicProperties::default().with_delivery_mode(2), // Persistent
        )
        .await?;
    Ok(())
}

pub async fn start_email_consumer(conn: Arc<lapin::Connection>) {
    tokio::spawn(async move {
        let channel = match conn.create_channel().await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to create MQ channel for consumer: {:?}", e);
                return;
            }
        };

        let mut consumer = match channel
            .basic_consume(
                "email_queue",
                "email_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
        {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to attach consumer to email_queue: {:?}", e);
                return;
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
                            // On error, triggering `requeue: false` bounces the frame onto the DLQ natively.
                            error!("Failed to send email. Bouncing to DLQ!");
                            let _ = delivery
                                .nack(BasicNackOptions {
                                    requeue: false,
                                    ..Default::default()
                                })
                                .await;
                        }
                    } else {
                        // Bad frame data, bouncing to DLQ.
                        let _ = delivery
                            .nack(BasicNackOptions {
                                requeue: false,
                                ..Default::default()
                            })
                            .await;
                    }
                }
                Err(error) => {
                    error!("Error within RabbitMQ consumer delivery stream: {:?}", error);
                }
            }
        }
    });
}

// Lettre execution resolving logic
async fn send_email_with_lettre(payload: &str) -> bool {
    // 1. Parse the JSON payload to extract email fields
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

    // 2. Build the email message
    AppConfig::init();
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
        .body(String::from(body))
    {
        Ok(msg) => msg,
        Err(e) => {
            tracing::error!("Failed to build email message: {:?}", e);
            return false;
        }
    };

    // 3. Setup the SMTP connection (e.g., Gmail, AWS SES, SendGrid)
    let creds = Credentials::new(cfg.smtp_username.clone(), cfg.smtp_password.clone());

    let mailer = SmtpTransport::relay("smtp.gmail.com")
        .unwrap()
        .credentials(creds)
        .build();

    // 4. Send and handle the result
    match mailer.send(&email) {
        Ok(_) => {
            tracing::info!("Email sent successfully to {}", to);
            true
        }
        Err(e) => {
            tracing::error!("SMTP delivery failed to {}: {:?}", to, e);
            false // Returning false triggers the DLQ bounce
        }
    }
}
