use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::post,
    Router,
};
use serde_json::{json, Value};
use tracing::warn;

use crate::{event::{CreateEventRequest, MovieEvent}, kafka::KafkaProducer};

type AppState = Arc<KafkaProducer>;

pub async fn serve(producer: Arc<KafkaProducer>, port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/events", post(publish_event))
        .with_state(producer);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("HTTP server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn publish_event(
    State(producer): State<AppState>,
    Json(req): Json<CreateEventRequest>,
) -> (StatusCode, Json<Value>) {
    let event = MovieEvent::from(req);

    if let Err(e) = event.validate() {
        warn!(error = %e, "Event validation failed");
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        );
    }

    let event_id = event.event_id.clone();

    match producer.publish(&event).await {
        Ok(_) => (
            StatusCode::ACCEPTED,
            Json(json!({ "event_id": event_id })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}