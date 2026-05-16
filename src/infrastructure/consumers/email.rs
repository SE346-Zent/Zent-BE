use std::sync::Arc;
use std::time::Duration;
use lapin::{
    options::{BasicConsumeOptions, BasicAckOptions, BasicNackOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use futures::stream::StreamExt;
use tracing::{info, error, warn};
use lettre::{Message, AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use lettre::transport::smtp::authentication::Credentials;
use tokio::time::{sleep};

use crate::core::config::AppConfig;
use crate::infrastructure::mq::email::{EMAIL_QUEUE, setup_email_topology};

/// Append `heartbeat=60` to the AMQP URL if not already present.
///
/// # Arguments
/// * `amqp_url` - The original RabbitMQ connection URL.
///
/// # Returns
/// A string containing the URL with the heartbeat parameter ensured.
fn ensure_heartbeat(amqp_url: &str) -> String {
    let heartbeat_param = "heartbeat=60";
    if amqp_url.contains("heartbeat=") {
        amqp_url.to_string()
    } else if amqp_url.contains('?') {
        format!("{}&{}", amqp_url, heartbeat_param)
    } else {
        format!("{}?{}", amqp_url, heartbeat_param)
    }
}

/// Attempt to establish a new asynchronous RabbitMQ connection.
///
/// # Arguments
/// * `amqp_url` - The RabbitMQ connection URL (e.g., amqp://user:pass@host).
///
/// # Returns
/// A result containing the `lapin::Connection` or a `lapin::Error`.
async fn create_fresh_connection(amqp_url: &str) -> Result<Connection, lapin::Error> {
    Connection::connect(&ensure_heartbeat(amqp_url), ConnectionProperties::default()).await
}

/// Initialize and start the background email consumer task.
///
/// This function spawns a long-running Tokio task that listens for email jobs
/// on RabbitMQ and processes them using `lettre`. It handles automatic
/// reconnection if the connection is lost.
///
/// # Arguments
/// * `amqp_connection` - An optional shared RabbitMQ connection. If `None`, the consumer remains idle (stub mode).
pub async fn start_email_consumer(amqp_connection: Option<Arc<lapin::Connection>>) {
    let mut conn_opt = match amqp_connection {
        Some(c) => c,
        None => return, // Stub mode
    };

    let url = AppConfig::get().rabbitmq_url.clone();

    tokio::spawn(async move {
        loop {
            let channel = match conn_opt.create_channel().await {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to create MQ channel for consumer: {:?}. Reconnecting...", e);
                    // Try to create a fresh connection — the old one is likely dead
                    match create_fresh_connection(&url).await {
                        Ok(new_conn) => {
                            info!("Email consumer established new RabbitMQ connection");
                            conn_opt = Arc::new(new_conn);
                            continue;
                        }
                        Err(re) => {
                            error!("Failed to reconnect email consumer: {:?}. Retrying in 5s...", re);
                            sleep(Duration::from_secs(5)).await;
                            continue;
                        }
                    }
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

/// Parse an email job payload and send the email using the Lettre SMTP transport.
///
/// # Arguments
/// * `json_payload` - A JSON string containing 'to', 'subject', and 'body' fields.
///
/// # Returns
/// `true` if the email was sent successfully, `false` otherwise.
async fn send_email_with_lettre(json_payload: &str) -> bool {
    let parsed: serde_json::Value = match serde_json::from_str(json_payload) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("Failed to parse email payload JSON: {:?}", e);
            return false;
        }
    };

    let to_address = match parsed["to"].as_str() {
        Some(v) => v,
        None => {
            tracing::error!("Missing 'to' field in email payload");
            return false;
        }
    };
    let email_subject = parsed["subject"].as_str().unwrap_or("System Notification");
    let html_body = parsed["body"].as_str().unwrap_or("");

    let cfg = AppConfig::get();
    let email_msg = match Message::builder()
        .from(format!("Zent System <{}>", cfg.smtp_username).parse().unwrap())
        .to(match to_address.parse() {
            Ok(addr) => addr,
            Err(e) => {
                tracing::error!("Invalid recipient email '{}': {:?}", to_address, e);
                return false;
            }
        })
        .subject(email_subject)
        .singlepart(lettre::message::SinglePart::html(String::from(html_body)))
    {
        Ok(msg) => msg,
        Err(e) => {
            tracing::error!("Failed to build email message: {:?}", e);
            return false;
        }
    };

    let smtp_credentials = Credentials::new(cfg.smtp_username.clone(), cfg.smtp_password.clone());

    let mailer: AsyncSmtpTransport<Tokio1Executor> = AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.gmail.com")
        .unwrap()
        .credentials(smtp_credentials)
        .build();

    match mailer.send(email_msg).await {
        Ok(_) => {
            tracing::info!("Email sent successfully to {}", to_address);
            true
        }
        Err(e) => {
            tracing::error!("SMTP delivery failed to {}: {:?}", to_address, e);
            false 
        }
    }
}
