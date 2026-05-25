use std::time::{Duration, Instant};

use movie_producer::{
    config::Config,
    event::{DeviceType, EventType, MovieEvent},
    kafka::KafkaProducer,
};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct ClickHouseRow {
    event_id: String,
    user_id: String,
    movie_id: String,
    event_type: String,
    progress_seconds: i32,
}

fn test_config() -> Config {
    Config {
        kafka_brokers: std::env::var("KAFKA_BROKERS").unwrap_or("localhost:9092".into()),
        schema_registry_url: std::env::var("SCHEMA_REGISTRY_URL")
            .unwrap_or("http://localhost:8081".into()),
        kafka_topic: "movie-events".into(),
        server_port: 3001,
        generator_enabled: false,
        generator_interval_ms: 500,
    }
}

#[tokio::test]
async fn test_event_flows_from_kafka_to_clickhouse() {
    let cfg = test_config();
    let producer = KafkaProducer::new(&cfg).expect("Failed to create producer");

    let event = MovieEvent::new(
        "test-user-integration".into(),
        "test-movie-integration".into(),
        EventType::ViewStarted,
        DeviceType::Desktop,
        "test-session-integration".into(),
        0,
    );
    let event_id = event.event_id.clone();

    producer.publish(&event).await.expect("Failed to publish event");

    let ch_url = std::env::var("CLICKHOUSE_URL").unwrap_or("http://localhost:8123".into());
    let client = reqwest::Client::new();
    let query = format!(
        "SELECT event_id, user_id, movie_id, event_type, progress_seconds \
         FROM cinema.movie_events WHERE event_id = '{}' FORMAT JSONEachRow",
        event_id
    );

    let deadline = Instant::now() + Duration::from_secs(15);
    let mut found = false;

    while Instant::now() < deadline {
        let text = match client.get(&ch_url).query(&[("query", &query)]).send().await {
            Ok(r) => r.text().await.unwrap_or_default(),
            Err(_) => String::new(),
        };

        if !text.trim().is_empty() {
            let row: ClickHouseRow =
                serde_json::from_str(text.trim()).expect("Failed to parse ClickHouse response");
            assert_eq!(row.event_id, event_id);
            assert_eq!(row.user_id, "test-user-integration");
            assert_eq!(row.movie_id, "test-movie-integration");
            assert_eq!(row.event_type, "VIEW_STARTED");
            assert_eq!(row.progress_seconds, 0);
            found = true;
            break;
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    assert!(found, "Event {event_id} not found in ClickHouse within 15 seconds");
}
