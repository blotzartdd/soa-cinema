use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{NaiveDate, Utc};
use sqlx::{PgPool, Row};
use tracing::warn;

use crate::metrics::DailyMetrics;

pub struct ExportRow {
    pub rank: i32,
    pub movie_id: String,
    pub view_count: i64,
}

pub struct PostgresClient {
    pool: PgPool,
}

impl PostgresClient {
    pub async fn connect(url: &str) -> Result<Self> {
        let pool = PgPool::connect(url)
            .await
            .context("Failed to connect to PostgreSQL")?;
        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .context("Failed to run PostgreSQL migrations")
    }

    pub async fn upsert_metrics(&self, m: &DailyMetrics) -> Result<()> {
        self.upsert_with_retry(m, 3).await
    }

    async fn upsert_with_retry(&self, m: &DailyMetrics, max_retries: u32) -> Result<()> {
        for attempt in 0..=max_retries {
            match self.do_upsert(m).await {
                Ok(()) => return Ok(()),
                Err(e) if attempt < max_retries => {
                    let backoff = Duration::from_millis(200 * (1u64 << attempt));
                    warn!(error = %e, attempt, "PostgreSQL upsert failed, retrying");
                    tokio::time::sleep(backoff).await;
                }
                Err(e) => return Err(e),
            }
        }
        unreachable!()
    }

    async fn do_upsert(&self, m: &DailyMetrics) -> Result<()> {
        let computed_at = Utc::now();

        let scalar = [
            ("dau", m.dau as f64),
            ("avg_watch_seconds", m.avg_watch_seconds),
            ("views_started", m.views_started as f64),
            ("views_finished", m.views_finished as f64),
            ("conversion_rate", m.conversion_rate),
            ("cohort_size", m.cohort_size as f64),
            ("returned_d1", m.returned_d1 as f64),
            ("returned_d7", m.returned_d7 as f64),
            ("retention_d1", m.retention_d1),
            ("retention_d7", m.retention_d7),
        ];

        for (name, value) in scalar {
            sqlx::query(
                "INSERT INTO daily_metrics (date, metric_name, metric_value, computed_at)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (date, metric_name) DO UPDATE
                 SET metric_value = EXCLUDED.metric_value, computed_at = EXCLUDED.computed_at",
            )
            .bind(m.date)
            .bind(name)
            .bind(value)
            .bind(computed_at)
            .execute(&self.pool)
            .await
            .with_context(|| format!("Failed to upsert metric '{name}'"))?;
        }

        for movie in &m.top_movies {
            sqlx::query(
                "INSERT INTO daily_top_movies (date, movie_id, view_count, rank, computed_at)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (date, movie_id) DO UPDATE
                 SET view_count = EXCLUDED.view_count, rank = EXCLUDED.rank, computed_at = EXCLUDED.computed_at",
            )
            .bind(m.date)
            .bind(&movie.movie_id)
            .bind(movie.view_count as i64)
            .bind(movie.rank as i32)
            .bind(computed_at)
            .execute(&self.pool)
            .await
            .context("Failed to upsert top movie")?;
        }

        Ok(())
    }

    pub async fn read_for_export(&self, date: NaiveDate) -> Result<(HashMap<String, f64>, Vec<ExportRow>)> {
        let metric_rows = sqlx::query(
            "SELECT metric_name, metric_value FROM daily_metrics WHERE date = $1",
        )
        .bind(date)
        .fetch_all(&self.pool)
        .await
        .context("Failed to read daily_metrics")?;

        let metrics: HashMap<String, f64> = metric_rows
            .iter()
            .map(|r| (r.get::<String, _>("metric_name"), r.get::<f64, _>("metric_value")))
            .collect();

        let movie_rows = sqlx::query(
            "SELECT rank, movie_id, view_count FROM daily_top_movies WHERE date = $1 ORDER BY rank",
        )
        .bind(date)
        .fetch_all(&self.pool)
        .await
        .context("Failed to read daily_top_movies")?;

        let top_movies = movie_rows
            .iter()
            .map(|r| ExportRow {
                rank: r.get("rank"),
                movie_id: r.get("movie_id"),
                view_count: r.get("view_count"),
            })
            .collect();

        Ok((metrics, top_movies))
    }
}
