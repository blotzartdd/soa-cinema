use std::time::Instant;

use anyhow::Result;
use chrono::NaiveDate;
use tracing::{error, info};

use crate::{
    clickhouse::ClickHouseClient,
    metrics::DailyMetrics,
    postgres::PostgresClient,
    s3_export::S3ExportClient,
};

pub struct AggregationResult {
    pub date: NaiveDate,
    pub total_events: u64,
    pub duration_ms: u64,
}

pub async fn run_for_date(
    date: NaiveDate,
    ch: &ClickHouseClient,
    pg: &PostgresClient,
    s3: &S3ExportClient,
) -> Result<AggregationResult> {
    let start = Instant::now();
    info!(date = %date, "Aggregation cycle started");

    let total_events = ch.query_total_events(date).await?;
    let dau = ch.query_dau(date).await?;
    let avg_watch_seconds = ch.query_avg_watch_time(date).await?;
    let top_movies = ch.query_top_movies(date).await?;
    let (views_started, views_finished, conversion_rate) = ch.query_conversion(date).await?;
    let (cohort_size, returned_d1, returned_d7, retention_d1, retention_d7) =
        ch.query_retention(date).await?;

    let metrics = DailyMetrics {
        date,
        dau,
        avg_watch_seconds,
        top_movies,
        views_started,
        views_finished,
        conversion_rate,
        cohort_size,
        returned_d1,
        returned_d7,
        retention_d1,
        retention_d7,
    };

    if let Err(e) = ch.write_aggregates(&metrics).await {
        error!(error = %e, "Failed to write aggregates to ClickHouse");
    }

    pg.upsert_metrics(&metrics).await?;

    if let Err(e) = s3.export_date(date, pg).await {
        error!(error = %e, date = %date, "S3 export failed");
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    info!(
        date = %date,
        total_events,
        duration_ms,
        dau,
        conversion_rate = format!("{:.2}%", conversion_rate * 100.0),
        retention_d1 = format!("{:.2}%", retention_d1 * 100.0),
        retention_d7 = format!("{:.2}%", retention_d7 * 100.0),
        "Aggregation cycle complete"
    );

    Ok(AggregationResult { date, total_events, duration_ms })
}
