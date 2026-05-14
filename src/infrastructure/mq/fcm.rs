use lapin::{
    options::{ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions, BasicPublishOptions},
    types::FieldTable,
    BasicProperties, ExchangeKind,
};
use std::sync::Arc;

pub const FCM_EXCHANGE: &str = "fcm_exchange";
pub const FCM_QUEUE: &str = "fcm_queue";
pub const FCM_ROUTING_KEY: &str = "send_push";
pub const FCM_DLX: &str = "fcm_dlx";
pub const FCM_DLQ: &str = "fcm_dlq";

pub async fn setup_fcm_topology(channel: &lapin::Channel) -> Result<(), lapin::Error> {
    // 1. Dead Letter Exchange and Queue
    channel.exchange_declare(
        FCM_DLX,
        ExchangeKind::Direct,
        ExchangeDeclareOptions { durable: true, ..Default::default() },
        FieldTable::default(),
    ).await?;

    channel.queue_declare(
        FCM_DLQ,
        QueueDeclareOptions { durable: true, ..Default::default() },
        FieldTable::default(),
    ).await?;

    channel.queue_bind(
        FCM_DLQ,
        FCM_DLX,
        FCM_ROUTING_KEY,
        QueueBindOptions::default(),
        FieldTable::default(),
    ).await?;

    // 2. Main Exchange and Queue with DLX attachment
    channel.exchange_declare(
        FCM_EXCHANGE,
        ExchangeKind::Direct,
        ExchangeDeclareOptions { durable: true, ..Default::default() },
        FieldTable::default(),
    ).await?;

    let mut queue_args = FieldTable::default();
    queue_args.insert(
        "x-dead-letter-exchange".into(),
        lapin::types::AMQPValue::LongString(FCM_DLX.into()),
    );

    channel.queue_declare(
        FCM_QUEUE,
        QueueDeclareOptions { durable: true, ..Default::default() },
        queue_args,
    ).await?;

    channel.queue_bind(
        FCM_QUEUE,
        FCM_EXCHANGE,
        FCM_ROUTING_KEY,
        QueueBindOptions::default(),
        FieldTable::default(),
    ).await?;

    Ok(())
}

pub struct FcmProducer {
    connection: Option<Arc<lapin::Connection>>,
}

impl FcmProducer {
    pub fn new(connection: Option<Arc<lapin::Connection>>) -> Self {
        Self { connection }
    }

    pub async fn publish(&self, payload: &[u8]) -> Result<(), anyhow::Error> {
        let conn = match &self.connection {
            Some(c) => c,
            None => return Ok(()), // Stub mode
        };

        let channel = conn.create_channel().await?;
        setup_fcm_topology(&channel).await?;

        let confirm = channel.basic_publish(
            FCM_EXCHANGE,
            FCM_ROUTING_KEY,
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
                Err(anyhow::anyhow!("Broker returned Nack for FCM message"))
            }
            Err(err) => {
                let _ = channel.close(200, "OK").await;
                Err(err.into())
            }
        }
    }
}
