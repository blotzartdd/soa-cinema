use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, Serializer};
use uuid::Uuid;

fn serialize_timestamp_ms<S: Serializer>(dt: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_i64(dt.timestamp_millis())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    ViewStarted,
    ViewFinished,
    ViewPaused,
    ViewResumed,
    Liked,
    Searched,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeviceType {
    Mobile,
    Desktop,
    Tv,
    Tablet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieEvent {
    pub event_id: String,
    pub user_id: String,
    pub movie_id: String,
    pub event_type: EventType,
    #[serde(serialize_with = "serialize_timestamp_ms")]
    pub timestamp: DateTime<Utc>,
    pub device_type: DeviceType,
    pub session_id: String,
    pub progress_seconds: i32,
}

impl MovieEvent {
    pub fn new(
        user_id: String,
        movie_id: String,
        event_type: EventType,
        device_type: DeviceType,
        session_id: String,
        progress_seconds: i32,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            user_id,
            movie_id,
            event_type,
            timestamp: Utc::now(),
            device_type,
            session_id,
            progress_seconds,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.event_id.is_empty() {
            return Err("event_id is required".into());
        }
        if self.user_id.is_empty() {
            return Err("user_id is required".into());
        }
        if self.movie_id.is_empty() {
            return Err("movie_id is required".into());
        }
        if self.session_id.is_empty() {
            return Err("session_id is required".into());
        }
        if self.progress_seconds < 0 {
            return Err("progress_seconds must be non-negative".into());
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateEventRequest {
    pub user_id: String,
    pub movie_id: String,
    pub event_type: EventType,
    pub device_type: DeviceType,
    pub session_id: String,
    pub progress_seconds: i32,
}

impl From<CreateEventRequest> for MovieEvent {
    fn from(req: CreateEventRequest) -> Self {
        MovieEvent::new(
            req.user_id,
            req.movie_id,
            req.event_type,
            req.device_type,
            req.session_id,
            req.progress_seconds,
        )
    }
}