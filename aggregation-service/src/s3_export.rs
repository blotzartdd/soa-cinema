use std::time::Duration;

use anyhow::{Context, Result};
use aws_sdk_s3::{
    config::{Credentials, Region},
    primitives::ByteStream,
    Client, Config,
};
use chrono::{NaiveDate, Utc};
use serde::Serialize;
use tracing::{info, warn};

use crate::postgres::PostgresClient;

pub struct S3ExportClient {
    client: Client,
    bucket: String,
}

#[derive(Serialize)]
struct ExportPayload<'a> {
    date: &'a str,
    exported_at: String,
    metrics: &'a std::collections::HashMap<String, f64>,
    top_movies: Vec<TopMovieExport<'a>>,
}

#[derive(Serialize)]
struct TopMovieExport<'a> {
    rank: i32,
    movie_id: &'a str,
    view_count: i64,
}

impl S3ExportClient {
    pub fn new(endpoint: &str, access_key: &str, secret_key: &str, bucket: &str) -> Self {
        let creds = Credentials::new(access_key, secret_key, None, None, "minio");
        let config = Config::builder()
            .endpoint_url(endpoint)
            .region(Region::new("us-east-1"))
            .credentials_provider(creds)
            .force_path_style(true)
            .behavior_version_latest()
            .build();
        Self {
            client: Client::from_conf(config),
            bucket: bucket.to_string(),
        }
    }

    pub async fn export_date(&self, date: NaiveDate, pg: &PostgresClient) -> Result<()> {
        const MAX_RETRIES: u32 = 3;
        for attempt in 0..=MAX_RETRIES {
            match self.do_export(date, pg).await {
                Ok(()) => return Ok(()),
                Err(e) if attempt < MAX_RETRIES => {
                    let backoff = Duration::from_millis(500 * (1u64 << attempt));
                    warn!(error = %e, attempt, "S3 export failed, retrying");
                    tokio::time::sleep(backoff).await;
                }
                Err(e) => return Err(e),
            }
        }
        unreachable!()
    }

    async fn do_export(&self, date: NaiveDate, pg: &PostgresClient) -> Result<()> {
        let (metrics, top_movies) = pg.read_for_export(date).await?;

        let date_str = date.to_string();
        let payload = ExportPayload {
            date: &date_str,
            exported_at: Utc::now().to_rfc3339(),
            metrics: &metrics,
            top_movies: top_movies
                .iter()
                .map(|m| TopMovieExport {
                    rank: m.rank,
                    movie_id: &m.movie_id,
                    view_count: m.view_count,
                })
                .collect(),
        };

        let json = serde_json::to_vec_pretty(&payload).context("Failed to serialize export")?;
        let key = format!("daily/{}/aggregates.json", date);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(ByteStream::from(json))
            .content_type("application/json")
            .send()
            .await
            .context("S3 put_object failed")?;

        info!(date = %date, key, "Exported aggregates to S3");
        Ok(())
    }
}
