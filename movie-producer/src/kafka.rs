use std::time::Duration;

use anyhow::Context;
use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
};
use tracing::{error, info, warn};

use crate::{config::Config, event::MovieEvent};

pub struct KafkaProducer {
    inner: FutureProducer,
    topic: String,
}

impl KafkaProducer {
    pub fn new(cfg: &Config) -> anyhow::Result<Self> {
        let inner: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &cfg.kafka_brokers)
            .set("message.timeout.ms", "10000")
            .set("acks", "all")
            .set("retries", "5")
            .set("retry.backoff.ms", "200")
            .set("enable.idempotence", "true")
            .create()
            .context("Failed to create Kafka producer")?;

        Ok(Self {
            inner,
            topic: cfg.kafka_topic.clone(),
        })
    }

    pub async fn publish(&self, event: &MovieEvent) -> anyhow::Result<()> {
        let payload = serde_json::to_string(event).context("Failed to serialize event")?;
        let key = event.user_id.as_str();

        const MAX_RETRIES: u32 = 5;
        let mut attempt = 0;

        loop {
            let record = FutureRecord::to(&self.topic)
                .payload(payload.as_bytes())
                .key(key);

            match self.inner.send(record, Duration::from_secs(5)).await {
                Ok((partition, offset)) => {
                    info!(
                        event_id = %event.event_id,
                        event_type = ?event.event_type,
                        timestamp = %event.timestamp,
                        partition,
                        offset,
                        "Published event to Kafka"
                    );
                    return Ok(());
                }
                Err((err, _)) if attempt < MAX_RETRIES => {
                    attempt += 1;
                    let backoff = Duration::from_millis(200 * (1 << attempt));
                    warn!(
                        error = %err,
                        attempt,
                        backoff_ms = backoff.as_millis(),
                        "Kafka publish failed, retrying"
                    );
                    tokio::time::sleep(backoff).await;
                }
                Err((err, _)) => {
                    error!(error = %err, "Kafka publish failed after all retries");
                    return Err(anyhow::anyhow!("Kafka publish error: {}", err));
                }
            }
        }
    }
}