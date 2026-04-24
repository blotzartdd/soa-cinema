CREATE TABLE IF NOT EXISTS daily_metrics (
    id          BIGSERIAL PRIMARY KEY,
    date        DATE NOT NULL,
    metric_name VARCHAR(100) NOT NULL,
    metric_value DOUBLE PRECISION NOT NULL,
    computed_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT daily_metrics_date_metric_key UNIQUE (date, metric_name)
);

CREATE TABLE IF NOT EXISTS daily_top_movies (
    id          BIGSERIAL PRIMARY KEY,
    date        DATE NOT NULL,
    movie_id    VARCHAR(255) NOT NULL,
    view_count  BIGINT NOT NULL,
    rank        INTEGER NOT NULL,
    computed_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT daily_top_movies_date_movie_key UNIQUE (date, movie_id)
);
