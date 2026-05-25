CREATE TABLE IF NOT EXISTS cinema.movie_events_kafka
(
    event_id        String,
    user_id         String,
    movie_id        String,
    event_type      LowCardinality(String),
    timestamp       DateTime64(3, 'UTC'),
    device_type     LowCardinality(String),
    session_id      String,
    progress_seconds Int32
)
ENGINE = Kafka
SETTINGS
    kafka_broker_list = 'kafka:29092,kafka-2:29093',
    kafka_topic_list = 'movie-events',
    kafka_group_name = 'clickhouse-consumer',
    kafka_format = 'JSONEachRow',
    kafka_num_consumers = 1,
    kafka_skip_broken_messages = 1;

CREATE TABLE IF NOT EXISTS cinema.movie_events
(
    event_id        String,
    user_id         String,
    movie_id        String,
    event_type      LowCardinality(String),
    timestamp       DateTime64(3, 'UTC'),
    device_type     LowCardinality(String),
    session_id      String,
    progress_seconds Int32,
    date            Date MATERIALIZED toDate(timestamp)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (user_id, timestamp, event_id)
SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW IF NOT EXISTS cinema.movie_events_mv
TO cinema.movie_events
AS SELECT
    event_id,
    user_id,
    movie_id,
    event_type,
    timestamp,
    device_type,
    session_id,
    progress_seconds
FROM cinema.movie_events_kafka;
