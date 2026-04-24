use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::warn;

use crate::{aggregator, clickhouse::ClickHouseClient, postgres::PostgresClient};

pub struct AppState {
    pub ch: Arc<ClickHouseClient>,
    pub pg: Arc<PostgresClient>,
}

pub async fn serve(state: Arc<AppState>, port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/aggregate", post(trigger_aggregate))
        .route("/health", get(health))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("HTTP server listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Deserialize)]
struct AggregateParams {
    date: Option<String>,
}

async fn trigger_aggregate(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AggregateParams>,
) -> (StatusCode, Json<Value>) {
    let date = match params.date {
        Some(s) => match NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => {
                warn!(date = %s, "Invalid date format in manual trigger");
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "invalid date format, use YYYY-MM-DD" })),
                );
            }
        },
        None => (Utc::now() - chrono::Duration::days(1)).date_naive(),
    };

    match aggregator::run_for_date(date, &state.ch, &state.pg).await {
        Ok(result) => (
            StatusCode::OK,
            Json(json!({
                "date": result.date.to_string(),
                "total_events": result.total_events,
                "duration_ms": result.duration_ms,
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
