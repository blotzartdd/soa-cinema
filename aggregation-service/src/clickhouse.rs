use anyhow::{Context, Result};
use chrono::{NaiveDate, Utc};
use reqwest::Client;
use serde::Deserialize;

use crate::metrics::{DailyMetrics, TopMovie};

pub struct ClickHouseClient {
    client: Client,
    url: String,
    db: String,
}

impl ClickHouseClient {
    pub fn new(url: &str, db: &str) -> Self {
        Self {
            client: Client::new(),
            url: url.to_string(),
            db: db.to_string(),
        }
    }

    async fn query_rows<T: for<'de> Deserialize<'de>>(&self, sql: &str) -> Result<Vec<T>> {
        let full_query = format!("{} FORMAT JSONEachRow", sql);
        let resp = self
            .client
            .post(&self.url)
            .query(&[
                ("database", self.db.as_str()),
                ("output_format_json_quote_64bit_integers", "0"),
            ])
            .body(full_query)
            .send()
            .await
            .context("ClickHouse query request failed")?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .context("Failed to read ClickHouse response")?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "ClickHouse query error ({}): {}",
                status,
                text
            ));
        }

        text.lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                serde_json::from_str(l)
                    .with_context(|| format!("Failed to parse ClickHouse row: {l}"))
            })
            .collect()
    }

    async fn insert_rows(&self, table: &str, rows: &[serde_json::Value]) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let body: String = rows
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect::<Vec<_>>()
            .join("\n");

        let query = format!("INSERT INTO {}.{} FORMAT JSONEachRow", self.db, table);
        let resp = self
            .client
            .post(&self.url)
            .query(&[("query", &query)])
            .body(body)
            .send()
            .await
            .context("ClickHouse insert request failed")?;

        let status = resp.status();
        if !status.is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "ClickHouse insert error ({}): {}",
                status,
                error_text
            ));
        }
        Ok(())
    }

    pub async fn query_total_events(&self, date: NaiveDate) -> Result<u64> {
        #[derive(Deserialize)]
        struct Row {
            total: u64,
        }
        let sql =
            format!("SELECT count() AS total FROM movie_events WHERE toDate(timestamp) = '{date}'");
        let rows: Vec<Row> = self.query_rows(&sql).await?;
        Ok(rows.into_iter().next().map(|r| r.total).unwrap_or(0))
    }

    pub async fn query_dau(&self, date: NaiveDate) -> Result<u64> {
        #[derive(Deserialize)]
        struct Row {
            dau: u64,
        }
        let sql = format!(
            "SELECT uniqExact(user_id) AS dau FROM movie_events WHERE toDate(timestamp) = '{date}'"
        );
        let rows: Vec<Row> = self.query_rows(&sql).await?;
        Ok(rows.into_iter().next().map(|r| r.dau).unwrap_or(0))
    }

    pub async fn query_avg_watch_time(&self, date: NaiveDate) -> Result<f64> {
        #[derive(Deserialize)]
        struct Row {
            avg_watch_seconds: f64,
        }
        let sql = format!(
            "SELECT avgOrDefault(progress_seconds) AS avg_watch_seconds \
             FROM movie_events \
             WHERE event_type = 'VIEW_FINISHED' AND toDate(timestamp) = '{date}'"
        );
        let rows: Vec<Row> = self.query_rows(&sql).await?;
        Ok(rows
            .into_iter()
            .next()
            .map(|r| r.avg_watch_seconds)
            .unwrap_or(0.0))
    }

    pub async fn query_top_movies(&self, date: NaiveDate) -> Result<Vec<TopMovie>> {
        #[derive(Deserialize)]
        struct Row {
            movie_id: String,
            view_count: u64,
        }
        let sql = format!(
            "SELECT movie_id, count() AS view_count \
             FROM movie_events \
             WHERE event_type = 'VIEW_STARTED' AND toDate(timestamp) = '{date}' \
             GROUP BY movie_id \
             ORDER BY view_count DESC \
             LIMIT 10"
        );
        let rows: Vec<Row> = self.query_rows(&sql).await?;
        Ok(rows
            .into_iter()
            .enumerate()
            .map(|(i, r)| TopMovie {
                movie_id: r.movie_id,
                view_count: r.view_count,
                rank: (i + 1) as u32,
            })
            .collect())
    }

    pub async fn query_conversion(&self, date: NaiveDate) -> Result<(u64, u64, f64)> {
        #[derive(Deserialize)]
        struct Row {
            started: u64,
            finished: u64,
            conversion_rate: f64,
        }
        let sql = format!(
            "SELECT \
               countIf(event_type = 'VIEW_STARTED') AS started, \
               countIf(event_type = 'VIEW_FINISHED') AS finished, \
               toFloat64(countIf(event_type = 'VIEW_FINISHED')) / \
                 greatest(toFloat64(countIf(event_type = 'VIEW_STARTED')), 1.0) * \
                 toFloat64(countIf(event_type = 'VIEW_STARTED') > 0) AS conversion_rate \
             FROM movie_events \
             WHERE toDate(timestamp) = '{date}' \
               AND event_type IN ('VIEW_STARTED', 'VIEW_FINISHED')"
        );
        let rows: Vec<Row> = self.query_rows(&sql).await?;
        let row = rows.into_iter().next().unwrap_or(Row {
            started: 0,
            finished: 0,
            conversion_rate: 0.0,
        });
        Ok((row.started, row.finished, row.conversion_rate))
    }

    pub async fn query_retention(&self, date: NaiveDate) -> Result<(u64, u64, u64, f64, f64)> {
        #[derive(Deserialize)]
        struct Row {
            cohort_size: u64,
            returned_d1: u64,
            returned_d7: u64,
            retention_d1: f64,
            retention_d7: f64,
        }
        let d1 = date + chrono::Duration::days(1);
        let d7 = date + chrono::Duration::days(7);
        let sql = format!(
            "WITH cohort AS (\
               SELECT user_id \
               FROM movie_events \
               WHERE event_type = 'VIEW_STARTED' \
               GROUP BY user_id \
               HAVING min(toDate(timestamp)) = '{date}'\
             ) \
             SELECT \
               count() AS cohort_size, \
               countIf(has_d1) AS returned_d1, \
               countIf(has_d7) AS returned_d7, \
               toFloat64(countIf(has_d1)) / greatest(toFloat64(count()), 1.0) * toFloat64(count() > 0) AS retention_d1, \
               toFloat64(countIf(has_d7)) / greatest(toFloat64(count()), 1.0) * toFloat64(count() > 0) AS retention_d7 \
             FROM (\
               SELECT \
                 c.user_id, \
                 countIf(toDate(e.timestamp) = '{d1}') > 0 AS has_d1, \
                 countIf(toDate(e.timestamp) = '{d7}') > 0 AS has_d7 \
               FROM cohort c \
               LEFT JOIN movie_events e ON c.user_id = e.user_id \
               GROUP BY c.user_id\
             )"
        );
        let rows: Vec<Row> = self.query_rows(&sql).await?;
        let row = rows.into_iter().next().unwrap_or(Row {
            cohort_size: 0,
            returned_d1: 0,
            returned_d7: 0,
            retention_d1: 0.0,
            retention_d7: 0.0,
        });
        Ok((
            row.cohort_size,
            row.returned_d1,
            row.returned_d7,
            row.retention_d1,
            row.retention_d7,
        ))
    }

    pub async fn write_aggregates(&self, m: &DailyMetrics) -> Result<()> {
        let computed_at = Utc::now().timestamp_millis();
        let date_str = m.date.to_string();

        let scalar_rows = vec![
            serde_json::json!({"date": date_str, "metric_name": "dau",              "metric_value": m.dau,              "computed_at": computed_at}),
            serde_json::json!({"date": date_str, "metric_name": "avg_watch_seconds","metric_value": m.avg_watch_seconds,"computed_at": computed_at}),
            serde_json::json!({"date": date_str, "metric_name": "views_started",    "metric_value": m.views_started,    "computed_at": computed_at}),
            serde_json::json!({"date": date_str, "metric_name": "views_finished",   "metric_value": m.views_finished,   "computed_at": computed_at}),
            serde_json::json!({"date": date_str, "metric_name": "conversion_rate",  "metric_value": m.conversion_rate,  "computed_at": computed_at}),
            serde_json::json!({"date": date_str, "metric_name": "cohort_size",      "metric_value": m.cohort_size,      "computed_at": computed_at}),
            serde_json::json!({"date": date_str, "metric_name": "returned_d1",      "metric_value": m.returned_d1,      "computed_at": computed_at}),
            serde_json::json!({"date": date_str, "metric_name": "returned_d7",      "metric_value": m.returned_d7,      "computed_at": computed_at}),
            serde_json::json!({"date": date_str, "metric_name": "retention_d1",     "metric_value": m.retention_d1,     "computed_at": computed_at}),
            serde_json::json!({"date": date_str, "metric_name": "retention_d7",     "metric_value": m.retention_d7,     "computed_at": computed_at}),
        ];
        self.insert_rows("agg_daily_metrics", &scalar_rows).await?;

        let movie_rows: Vec<serde_json::Value> = m
            .top_movies
            .iter()
            .map(|movie| {
                serde_json::json!({
                    "date": date_str,
                    "movie_id": movie.movie_id,
                    "view_count": movie.view_count,
                    "rank": movie.rank,
                    "computed_at": computed_at,
                })
            })
            .collect();
        self.insert_rows("agg_top_movies", &movie_rows).await?;

        Ok(())
    }
}
