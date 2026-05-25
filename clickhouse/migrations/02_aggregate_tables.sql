CREATE TABLE IF NOT EXISTS cinema.agg_daily_metrics
(
    date         Date,
    metric_name  LowCardinality(String),
    metric_value Float64,
    computed_at  DateTime64(3, 'UTC')
)
ENGINE = ReplacingMergeTree(computed_at)
ORDER BY (date, metric_name);

CREATE TABLE IF NOT EXISTS cinema.agg_top_movies
(
    date        Date,
    movie_id    String,
    view_count  UInt64,
    rank        UInt32,
    computed_at DateTime64(3, 'UTC')
)
ENGINE = ReplacingMergeTree(computed_at)
ORDER BY (date, movie_id);
