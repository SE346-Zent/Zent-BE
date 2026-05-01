use std::sync::Arc;
use lapin::{
    options::{BasicConsumeOptions, BasicAckOptions, BasicNackOptions},
    types::FieldTable,
};
use futures::stream::StreamExt;
use tracing::{info, error, warn};
use lettre::{Message, AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use lettre::transport::smtp::authentication::Credentials;
use tokio::time::{sleep, Duration};

use crate::core::config::AppConfig;
use crate::infrastructure::mq::{RabbitMQClient, email::{EMAIL_QUEUE, setup_email_topology}};

pub async fn start_email_consumer(manager: Arc<RabbitMQClient>) {
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

            // Ensure topology is set up (Idempotent)
            if let Err(e) = setup_email_topology(&channel).await {
                error!("Failed to setup email topology for consumer: {:?}. Retrying in 5s...", e);
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            let mut consumer = match channel.basic_consume(
                EMAIL_QUEUE,
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

    let mailer: AsyncSmtpTransport<Tokio1Executor> = AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.gmail.com")
        .unwrap()
        .credentials(creds)
        .build();

    match mailer.send(email).await {
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
