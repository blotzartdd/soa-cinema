mod aggregator;
mod api;
mod clickhouse;
mod config;
mod metrics;
mod postgres;
mod s3_export;
mod scheduler;

use std::sync::Arc;

use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_env("RUST_LOG")
                .add_directive("aggregation_service=info".parse().unwrap()),
        )
        .init();

    let cfg = config::Config::from_env();
    info!(
        clickhouse = %cfg.clickhouse_url,
        schedule_interval_secs = cfg.schedule_interval_secs,
        "Starting aggregation-service"
    );

    let pg = postgres::PostgresClient::connect(&cfg.postgres_url).await?;
    pg.run_migrations().await?;
    info!("PostgreSQL migrations applied");

    let ch = clickhouse::ClickHouseClient::new(&cfg.clickhouse_url, &cfg.clickhouse_db);
    let s3 = s3_export::S3ExportClient::new(
        &cfg.s3_endpoint,
        &cfg.s3_access_key,
        &cfg.s3_secret_key,
        &cfg.s3_bucket,
    );

    let state = Arc::new(api::AppState {
        ch: Arc::new(ch),
        pg: Arc::new(pg),
        s3: Arc::new(s3),
    });

    scheduler::start(state.clone(), cfg.schedule_interval_secs);
    info!("Scheduler started, interval={}s", cfg.schedule_interval_secs);

    api::serve(state, cfg.server_port).await
}
