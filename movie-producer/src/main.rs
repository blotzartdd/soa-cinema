use movie_producer::{api, config, generator, kafka};

use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_env("RUST_LOG")
                .add_directive("movie_producer=info".parse().unwrap()),
        )
        .init();

    let cfg = config::Config::from_env();
    info!("Starting movie-producer, broker={}", cfg.kafka_brokers);

    let producer = Arc::new(kafka::KafkaProducer::new(&cfg)?);

    if cfg.generator_enabled {
        let gen_producer = producer.clone();
        let interval_ms = cfg.generator_interval_ms;
        tokio::spawn(async move {
            generator::run(gen_producer, interval_ms).await;
        });
        info!("Event generator started, interval={}ms", cfg.generator_interval_ms);
    }

    api::serve(producer, cfg.server_port).await
}