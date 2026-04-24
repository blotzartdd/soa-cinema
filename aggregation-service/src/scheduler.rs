use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tracing::error;

use crate::{aggregator, api::AppState};

pub fn start(state: Arc<AppState>, interval_secs: u64) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            let date = (Utc::now() - chrono::Duration::days(1)).date_naive();
            if let Err(e) = aggregator::run_for_date(date, &state.ch, &state.pg).await {
                error!(error = %e, date = %date, "Scheduled aggregation failed");
            }
        }
    });
}
