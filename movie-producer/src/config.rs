pub struct Config {
    pub kafka_brokers: String,
    pub schema_registry_url: String,
    pub kafka_topic: String,
    pub server_port: u16,
    pub generator_enabled: bool,
    pub generator_interval_ms: u64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            kafka_brokers: std::env::var("KAFKA_BROKERS")
                .unwrap_or_else(|_| "localhost:9092".to_string()),
            schema_registry_url: std::env::var("SCHEMA_REGISTRY_URL")
                .unwrap_or_else(|_| "http://localhost:8081".to_string()),
            kafka_topic: std::env::var("KAFKA_TOPIC")
                .unwrap_or_else(|_| "movie-events".to_string()),
            server_port: std::env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .unwrap_or(3000),
            generator_enabled: std::env::var("GENERATOR_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                == "true",
            generator_interval_ms: std::env::var("GENERATOR_INTERVAL_MS")
                .unwrap_or_else(|_| "500".to_string())
                .parse()
                .unwrap_or(500),
        }
    }
}