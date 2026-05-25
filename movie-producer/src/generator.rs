use std::sync::Arc;

use rand::{seq::SliceRandom, Rng};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    event::{DeviceType, EventType, MovieEvent},
    kafka::KafkaProducer,
};

static USER_IDS: &[&str] = &[
    "user-001", "user-002", "user-003", "user-004", "user-005",
    "user-006", "user-007", "user-008", "user-009", "user-010",
];

static MOVIE_IDS: &[&str] = &[
    "movie-inception", "movie-interstellar", "movie-matrix",
    "movie-dark-knight", "movie-pulp-fiction", "movie-godfather",
];

static DEVICES: &[DeviceType] = &[DeviceType::Mobile, DeviceType::Desktop, DeviceType::Tv, DeviceType::Tablet];

pub async fn run(producer: Arc<KafkaProducer>, interval_ms: u64) {
    let interval = tokio::time::Duration::from_millis(interval_ms);

    loop {
        let events = {
            let mut rng = rand::thread_rng();
            generate_session(&mut rng)
        };
        for event in events {
            if let Err(e) = producer.publish(&event).await {
                error!(error = %e, "Generator failed to publish event");
            }
        }
        tokio::time::sleep(interval).await;
    }
}

fn generate_session(rng: &mut impl Rng) -> Vec<MovieEvent> {
    let user_id = USER_IDS.choose(rng).unwrap().to_string();
    let movie_id = MOVIE_IDS.choose(rng).unwrap().to_string();
    let session_id = Uuid::new_v4().to_string();
    let device = DEVICES.choose(rng).unwrap().clone();
    let movie_duration: i32 = rng.gen_range(3600..7200);

    let mut events = Vec::new();
    let mut progress: i32 = 0;

    events.push(MovieEvent::new(
        user_id.clone(), movie_id.clone(), EventType::ViewStarted,
        device.clone(), session_id.clone(), progress,
    ));

    let pauses = rng.gen_range(0..3usize);
    for _ in 0..pauses {
        progress += rng.gen_range(60..600);
        progress = progress.min(movie_duration - 60);
        events.push(MovieEvent::new(
            user_id.clone(), movie_id.clone(), EventType::ViewPaused,
            device.clone(), session_id.clone(), progress,
        ));
        progress += rng.gen_range(1..60);
        events.push(MovieEvent::new(
            user_id.clone(), movie_id.clone(), EventType::ViewResumed,
            device.clone(), session_id.clone(), progress,
        ));
    }

    if rng.gen_bool(0.75) {
        events.push(MovieEvent::new(
            user_id.clone(), movie_id.clone(), EventType::ViewFinished,
            device.clone(), session_id.clone(), movie_duration,
        ));
    }

    if rng.gen_bool(0.30) {
        events.push(MovieEvent::new(
            user_id.clone(), movie_id.clone(), EventType::Liked,
            device.clone(), session_id.clone(), 0,
        ));
    }

    if rng.gen_bool(0.20) {
        let random_movie = MOVIE_IDS.choose(rng).unwrap().to_string();
        events.push(MovieEvent::new(
            user_id.clone(), random_movie, EventType::Searched,
            device.clone(), session_id.clone(), 0,
        ));
    }

    info!(
        user_id = %user_id,
        movie_id = %movie_id,
        session_events = events.len(),
        "Generated synthetic session"
    );

    events
}