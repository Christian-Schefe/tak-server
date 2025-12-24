use std::{sync::Arc, time::Duration};

use crate::app::domain::r#match::MatchService;

pub struct MatchCleanupJob<M: MatchService> {
    match_service: Arc<M>,
}

impl<M: MatchService + Send + Sync + 'static> MatchCleanupJob<M> {
    pub fn new(match_service: Arc<M>) -> Self {
        Self { match_service }
    }

    pub async fn run(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(60 * 60));
        loop {
            self.match_service
                .cleanup_old_matches(std::time::Instant::now());
            interval.tick().await;
        }
    }
}
