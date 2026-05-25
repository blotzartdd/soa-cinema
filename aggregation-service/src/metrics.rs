use chrono::NaiveDate;

#[derive(Debug, Clone)]
pub struct TopMovie {
    pub movie_id: String,
    pub view_count: u64,
    pub rank: u32,
}

#[derive(Debug, Clone)]
pub struct DailyMetrics {
    pub date: NaiveDate,
    pub dau: u64,
    pub avg_watch_seconds: f64,
    pub top_movies: Vec<TopMovie>,
    pub views_started: u64,
    pub views_finished: u64,
    pub conversion_rate: f64,
    pub cohort_size: u64,
    pub returned_d1: u64,
    pub returned_d7: u64,
    pub retention_d1: f64,
    pub retention_d7: f64,
}
