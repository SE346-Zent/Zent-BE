use lapin::{
    options::{ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions, BasicPublishOptions},
    types::FieldTable,
    BasicProperties, ExchangeKind,
};
use std::sync::Arc;

pub const NOTIFICATION_EXCHANGE: &str = "notification_exchange";
pub const NOTIFICATION_QUEUE: &str = "notification_queue";
pub const NOTIFICATION_ROUTING_KEY: &str = "notification.created";
pub const NOTIFICATION_DLX: &str = "notification_dlx";
pub const NOTIFICATION_DLQ: &str = "notification_dlq";

pub async fn setup_notification_topology(channel: &lapin::Channel) -> Result<(), lapin::Error> {
    // 1. Dead Letter Exchange and Queue
    channel.exchange_declare(
        NOTIFICATION_DLX,
        ExchangeKind::Direct,
        ExchangeDeclareOptions { durable: true, ..Default::default() },
        FieldTable::default(),
    ).await?;

    channel.queue_declare(
        NOTIFICATION_DLQ,
        QueueDeclareOptions { durable: true, ..Default::default() },
        FieldTable::default(),
    ).await?;

    channel.queue_bind(
        NOTIFICATION_DLQ,
        NOTIFICATION_DLX,
        NOTIFICATION_ROUTING_KEY,
        QueueBindOptions::default(),
        FieldTable::default(),
    ).await?;

    // 2. Main Exchange and Queue with DLX attachment
    channel.exchange_declare(
        NOTIFICATION_EXCHANGE,
        ExchangeKind::Direct,
        ExchangeDeclareOptions { durable: true, ..Default::default() },
        FieldTable::default(),
    ).await?;

    let mut queue_args = FieldTable::default();
    queue_args.insert(
        "x-dead-letter-exchange".into(),
        lapin::types::AMQPValue::LongString(NOTIFICATION_DLX.into()),
    );

    channel.queue_declare(
        NOTIFICATION_QUEUE,
        QueueDeclareOptions { durable: true, ..Default::default() },
        queue_args,
    ).await?;

    channel.queue_bind(
        NOTIFICATION_QUEUE,
        NOTIFICATION_EXCHANGE,
        NOTIFICATION_ROUTING_KEY,
        QueueBindOptions::default(),
        FieldTable::default(),
    ).await?;

    Ok(())
}

pub struct NotificationProducer {
    connection: Option<Arc<lapin::Connection>>,
}

impl NotificationProducer {
    pub fn new(connection: Option<Arc<lapin::Connection>>) -> Self {
        Self { connection }
    }

    pub async fn publish(&self, payload: &[u8]) -> Result<(), anyhow::Error> {
        let conn = match &self.connection {
            Some(c) => c,
            None => return Ok(()), // Stub mode
        };

        let channel = conn.create_channel().await?;
        setup_notification_topology(&channel).await?;

        let confirm = channel.basic_publish(
            NOTIFICATION_EXCHANGE,
            NOTIFICATION_ROUTING_KEY,
            BasicPublishOptions::default(),
            payload,
            BasicProperties::default().with_delivery_mode(2), // Persistent
        ).await?;

        match confirm.await {
            Ok(lapin::publisher_confirm::Confirmation::Ack(_)) | Ok(lapin::publisher_confirm::Confirmation::NotRequested) => {
                let _ = channel.close(200, "OK").await;
                Ok(())
            }
            Ok(lapin::publisher_confirm::Confirmation::Nack(_)) => {
                let _ = channel.close(200, "OK").await;
                Err(anyhow::anyhow!("Broker returned Nack for notification message"))
            }
            Err(err) => {
                let _ = channel.close(200, "OK").await;
                Err(err.into())
            }
        }
    }
}
