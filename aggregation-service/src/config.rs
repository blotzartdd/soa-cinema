pub struct Config {
    pub clickhouse_url: String,
    pub clickhouse_db: String,
    pub postgres_url: String,
    pub server_port: u16,
    pub schedule_interval_secs: u64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            clickhouse_url: std::env::var("CLICKHOUSE_URL")
                .unwrap_or_else(|_| "http://localhost:8123".to_string()),
            clickhouse_db: std::env::var("CLICKHOUSE_DB")
                .unwrap_or_else(|_| "cinema".to_string()),
            postgres_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| {
                    "postgres://analytics:analytics_pass@localhost/cinema_analytics".to_string()
                }),
            server_port: std::env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3001".to_string())
                .parse()
                .unwrap_or(3001),
            schedule_interval_secs: std::env::var("SCHEDULE_INTERVAL_SECS")
                .unwrap_or_else(|_| "3600".to_string())
                .parse()
                .unwrap_or(3600),
        }
    }
}
