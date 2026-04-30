use lapin::{
    options::{ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions, BasicPublishOptions},
    types::FieldTable,
    BasicProperties, ExchangeKind,
};
use std::sync::Arc;
use crate::infrastructure::mq::RabbitMQManager;

pub const EMAIL_EXCHANGE: &str = "email_exchange";
pub const EMAIL_QUEUE: &str = "email_queue";
pub const EMAIL_ROUTING_KEY: &str = "send_email";
pub const EMAIL_DLX: &str = "email_dlx";
pub const EMAIL_DLQ: &str = "email_dlq";

pub async fn setup_email_topology(channel: &lapin::Channel) -> Result<(), lapin::Error> {
    // 1. Declare Dead Letter Exchange and Queue
    channel.exchange_declare(
        EMAIL_DLX,
        ExchangeKind::Direct,
        ExchangeDeclareOptions { durable: true, ..Default::default() },
        FieldTable::default(),
    ).await?;

    channel.queue_declare(
        EMAIL_DLQ,
        QueueDeclareOptions { durable: true, ..Default::default() },
        FieldTable::default(),
    ).await?;

    channel.queue_bind(
        EMAIL_DLQ,
        EMAIL_DLX,
        EMAIL_ROUTING_KEY,
        QueueBindOptions::default(),
        FieldTable::default(),
    ).await?;

    // 2. Declare Main Exchange and Queue attaching DLX fallbacks
    channel.exchange_declare(
        EMAIL_EXCHANGE,
        ExchangeKind::Direct,
        ExchangeDeclareOptions { durable: true, ..Default::default() },
        FieldTable::default(),
    ).await?;

    let mut queue_args = FieldTable::default();
    queue_args.insert(
        "x-dead-letter-exchange".into(),
        lapin::types::AMQPValue::LongString(EMAIL_DLX.into()),
    );

    channel.queue_declare(
        EMAIL_QUEUE,
        QueueDeclareOptions { durable: true, ..Default::default() },
        queue_args,
    ).await?;

    channel.queue_bind(
        EMAIL_QUEUE,
        EMAIL_EXCHANGE,
        EMAIL_ROUTING_KEY,
        QueueBindOptions::default(),
        FieldTable::default(),
    ).await?;

    Ok(())
}

pub struct EmailProducer {
    manager: Arc<RabbitMQManager>,
}

impl EmailProducer {
    pub fn new(manager: Arc<RabbitMQManager>) -> Self {
        Self { manager }
    }

    pub async fn publish(&self, payload: &[u8]) -> Result<(), anyhow::Error> {
        let meter = crate::infrastructure::observability::meter();
        let publish_count = meter.u64_counter("messaging.publish.count").build();
        let publish_errors = meter.u64_counter("messaging.publish.errors").build();

        if self.manager.is_stub() {
            return Ok(());
        }

        let conn = self.manager.get_connection().await?;
        let channel = conn.create_channel().await?;
        
        // Ensure topology is set up (Idempotent)
        setup_email_topology(&channel).await?;

        match channel.basic_publish(
            EMAIL_EXCHANGE,
            EMAIL_ROUTING_KEY,
            BasicPublishOptions::default(),
            payload,
            BasicProperties::default().with_delivery_mode(2), // Persistent
        ).await {
            Ok(_) => {
                publish_count.add(1, &[opentelemetry::KeyValue::new("exchange", EMAIL_EXCHANGE)]);
                let _ = channel.close(200, "OK").await;
                Ok(())
            }
            Err(err) => {
                publish_errors.add(1, &[opentelemetry::KeyValue::new("exchange", EMAIL_EXCHANGE)]);
                Err(err.into())
            }
        }
    }
}
