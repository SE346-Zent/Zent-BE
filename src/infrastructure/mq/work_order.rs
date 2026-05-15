use lapin::{
    options::{ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions, BasicPublishOptions},
    types::FieldTable,
    BasicProperties, ExchangeKind, ConnectionProperties,
};
use tracing::warn;
use crate::core::config::AppConfig;
use std::sync::Arc;

pub const WORK_ORDER_EXCHANGE: &str = "work_order_exchange";
pub const WORK_ORDER_CREATED_QUEUE: &str = "work_order_created_queue";
pub const WORK_ORDER_CREATED_ROUTING_KEY: &str = "work_order.created";
pub const WORK_ORDER_DLX: &str = "work_order_dlx";
pub const WORK_ORDER_DLQ: &str = "work_order_dlq";

pub async fn setup_work_order_topology(channel: &lapin::Channel) -> Result<(), lapin::Error> {
    // 1. DLX and DLQ
    channel.exchange_declare(
        WORK_ORDER_DLX,
        ExchangeKind::Direct,
        ExchangeDeclareOptions { durable: true, ..Default::default() },
        FieldTable::default(),
    ).await?;

    channel.queue_declare(
        WORK_ORDER_DLQ,
        QueueDeclareOptions { durable: true, ..Default::default() },
        FieldTable::default(),
    ).await?;

    channel.queue_bind(
        WORK_ORDER_DLQ,
        WORK_ORDER_DLX,
        WORK_ORDER_CREATED_ROUTING_KEY,
        QueueBindOptions::default(),
        FieldTable::default(),
    ).await?;

    // 2. Main Exchange and Queue
    channel.exchange_declare(
        WORK_ORDER_EXCHANGE,
        ExchangeKind::Topic,
        ExchangeDeclareOptions { durable: true, ..Default::default() },
        FieldTable::default(),
    ).await?;

    let mut queue_args = FieldTable::default();
    queue_args.insert(
        "x-dead-letter-exchange".into(),
        lapin::types::AMQPValue::LongString(WORK_ORDER_DLX.into()),
    );

    channel.queue_declare(
        WORK_ORDER_CREATED_QUEUE,
        QueueDeclareOptions { durable: true, ..Default::default() },
        queue_args,
    ).await?;

    channel.queue_bind(
        WORK_ORDER_CREATED_QUEUE,
        WORK_ORDER_EXCHANGE,
        WORK_ORDER_CREATED_ROUTING_KEY,
        QueueBindOptions::default(),
        FieldTable::default(),
    ).await?;

    Ok(())
}

pub struct WorkOrderProducer {
    connection: Option<Arc<lapin::Connection>>,
}

impl WorkOrderProducer {
    pub fn new(connection: Option<Arc<lapin::Connection>>) -> Self {
        Self { connection }
    }

    pub async fn publish_created(&self, payload: &[u8]) -> Result<(), anyhow::Error> {
        let conn = match &self.connection {
            Some(c) => c,
            None => return Ok(()),
        };

        // Fast path: try with the shared connection
        match publish_created_on(conn, payload).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                warn!("Work order publish on shared connection failed: {}. Retrying with fresh connection...", e);
            }
        }

        // Slow path: shared connection is stale (e.g. after consumer reconnect).
        // Create a fresh connection for this one publish.
        let url = super::ensure_heartbeat(&AppConfig::get().rabbitmq_url);
        let fresh_conn = lapin::Connection::connect(&url, ConnectionProperties::default()).await
            .map_err(|e| anyhow::anyhow!("Failed to create fresh connection: {}", e))?;
        publish_created_on(&fresh_conn, payload).await
    }
}

async fn publish_created_on(conn: &lapin::Connection, payload: &[u8]) -> Result<(), anyhow::Error> {
    let channel = conn.create_channel().await?;
    setup_work_order_topology(&channel).await?;

    let confirm = channel.basic_publish(
        WORK_ORDER_EXCHANGE,
        WORK_ORDER_CREATED_ROUTING_KEY,
        BasicPublishOptions::default(),
        payload,
        BasicProperties::default().with_delivery_mode(2),
    ).await?;

    match confirm.await {
        Ok(lapin::publisher_confirm::Confirmation::Ack(_)) | Ok(lapin::publisher_confirm::Confirmation::NotRequested) => {
            let _ = channel.close(200, "OK").await;
            Ok(())
        }
        Ok(lapin::publisher_confirm::Confirmation::Nack(_)) => {
            let _ = channel.close(200, "OK").await;
            Err(anyhow::anyhow!("Broker returned Nack"))
        }
        Err(err) => {
            let _ = channel.close(200, "OK").await;
            Err(err.into())
        }
    }
}
