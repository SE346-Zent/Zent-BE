use futures::stream::StreamExt;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, BasicNackOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tokio_executor_trait::Tokio as TokioExecutor;
use tracing::{error, info, warn};

use crate::core::config::AppConfig;
use crate::infrastructure::mq::email::{setup_email_topology, EMAIL_QUEUE};

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
                    error!(
                        message = "AMQP channel creation failed for email consumer",
                        error.message = %e,
                        error.details = ?e,
                        "Attempting connection fallback recovery"
                    );

                    match create_fresh_connection(&url).await {
                        Ok(new_conn) => {
                            info!(
                                message = "RabbitMQ connection re-established",
                                "Resuming email consumer execution loop"
                            );
                            conn_opt = Arc::new(new_conn);
                            continue;
                        }
                        Err(re) => {
                            error!(
                                message = "RabbitMQ connection recovery failed",
                                error.message = %re,
                                error.details = ?re,
                                "Retrying connection recovery loop in 5 seconds"
                            );
                            sleep(Duration::from_secs(5)).await;
                            continue;
                        }
                    }
                }
            };

            if let Err(e) = setup_email_topology(&channel).await {
                error!(
                    message = "AMQP topology setup failed for email queue",
                    error.message = %e,
                    error.details = ?e,
                    "Retrying topology registration in 5 seconds"
                );
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            let mut consumer = match channel
                .basic_consume(
                    EMAIL_QUEUE,
                    "email_consumer",
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    error!(
                        message = "AMQP consumer binding failed for email queue",
                        error.message = %e,
                        error.details = ?e,
                        queue = %EMAIL_QUEUE,
                        "Retrying queue consumption in 5 seconds"
                    );
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            info!(
                message = "Email consumer stream activated",
                queue = %EMAIL_QUEUE,
                "Awaiting inbound message frames"
            );

            while let Some(delivery) = consumer.next().await {
                match delivery {
                    Ok(delivery) => {
                        if let Ok(payload) = std::str::from_utf8(&delivery.data) {
                            info!(
                                message = "Inbound email frame received",
                                payload_raw = %payload,
                                "Beginning delivery processing"
                            );

                            let success = send_email_with_lettre(payload).await;

                            if success {
                                let _ = delivery.ack(BasicAckOptions::default()).await;
                            } else {
                                error!(
                                    message = "Email processing pipeline failed",
                                    "Rejecting message frame to dead letter queue"
                                );
                                let _ = delivery
                                    .nack(BasicNackOptions {
                                        requeue: false,
                                        ..Default::default()
                                    })
                                    .await;
                            }
                        } else {
                            error!(
                                message = "Inbound queue frame contains invalid UTF-8 data",
                                "Rejecting unreadable message payload"
                            );
                            let _ = delivery
                                .nack(BasicNackOptions {
                                    requeue: false,
                                    ..Default::default()
                                })
                                .await;
                        }
                    }
                    Err(e) => {
                        error!(
                            message = "AMQP delivery stream connection severed",
                            error.message = %e,
                            error.details = ?e,
                            "Breaking trace stream consumer loop"
                        );
                        break;
                    }
                }
            }

            warn!(
                message = "Email consumer transaction loop broken unexpectly",
                "Initiating restart cooling delay for 5 seconds"
            );
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
            error!(
                message = "Payload deserialization failed",
                error.message = %e,
                error.details = ?e,
                "Aborting email dispatch"
            );
            return false;
        }
    };

    let to_address = match parsed["to"].as_str() {
        Some(v) => v,
        None => {
            error!(
                message = "Validation failed: missing recipient address",
                "Aborting email dispatch"
            );
            return false;
        }
    };
    let email_subject = parsed["subject"].as_str().unwrap_or("System Notification");
    let html_body = parsed["body"].as_str().unwrap_or("");

    let cfg = AppConfig::get();
    let email_msg = match Message::builder()
        .from(
            format!("Zent System <{}>", cfg.smtp_username)
                .parse()
                .unwrap(),
        )
        .to(match to_address.parse() {
            Ok(addr) => addr,
            Err(e) => {
                error!(
                    message = "Validation failed: invalid recipient syntax",
                    error.message = %e,
                    error.details = ?e,
                    recipient = %to_address,
                    "Aborting email dispatch"
                );
                return false;
            }
        })
        .subject(email_subject)
        .singlepart(lettre::message::SinglePart::html(String::from(html_body)))
    {
        Ok(msg) => msg,
        Err(e) => {
            error!(
                message = "Lettre message compilation failed",
                error.message = %e,
                error.details = ?e,
                recipient = %to_address,
                "Aborting email dispatch"
            );
            return false;
        }
    };

    let smtp_credentials = Credentials::new(cfg.smtp_username.clone(), cfg.smtp_password.clone());

    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        match AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.gmail.com") {
            Ok(builder) => builder.credentials(smtp_credentials).build(),
            Err(e) => {
                error!(
                    message = "SMTP mailer transport building failed",
                    error.message = %e,
                    error.details = ?e,
                    recipient = %to_address,
                    "Aborting email dispatch"
                );
                return false;
            }
        };

    match mailer.send(email_msg).await {
        Ok(_) => {
            info!(
                message = "Email dispatched successfully",
                recipient = %to_address,
                subject = %email_subject,
                "SMTP transaction completed"
            );
            true
        }
        Err(e) => {
            error!(
                message = "SMTP delivery failure",
                error.message = %e,
                error.details = ?e,
                recipient = %to_address,
                subject = %email_subject,
                "Aborting transaction"
            );
            false
        }
    }
}
