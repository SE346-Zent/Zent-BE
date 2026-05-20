use lapin::{
    options::{ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions, BasicPublishOptions},
    types::FieldTable,
    BasicProperties, ExchangeKind, ConnectionProperties,
};
use tokio_executor_trait::Tokio as TokioExecutor;
use crate::core::config::AppConfig;

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

/// Producer that publishes work-order-created events to RabbitMQ.
///
/// Always creates a fresh connection per publish to avoid the stale-shared-connection
/// problem: the consumer reconnects independently and updates its own handle, but the
/// shared `AppState::rabbitmq` is never refreshed. Publishing on the stale connection
/// can silently drop messages (lapin reports success on a dead connection).
pub struct WorkOrderProducer;

impl WorkOrderProducer {
    pub fn new() -> Self {
        Self
    }

    pub async fn publish_created(&self, payload: &[u8]) -> Result<(), anyhow::Error> {
        let url = super::ensure_heartbeat(&AppConfig::get().rabbitmq_url);
        let conn = lapin::Connection::connect(&url, ConnectionProperties::default().with_executor(TokioExecutor::current())).await
            .map_err(|e| anyhow::anyhow!("Failed to connect to RabbitMQ for WO publish: {}", e))?;
        publish_created_on(&conn, payload).await
    }
}

async fn publish_created_on(conn: &lapin::Connection, payload: &[u8]) -> Result<(), anyhow::Error> {
    let channel = conn.create_channel().await?;
    setup_work_order_topology(&channel).await?;

    // Enable publisher confirms so we know the broker received the message
    use lapin::options::ConfirmSelectOptions;
    channel.confirm_select(ConfirmSelectOptions::default()).await?;

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
