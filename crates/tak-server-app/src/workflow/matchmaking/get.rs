use std::sync::Arc;

use crate::{
    domain::{
        MatchId, RepoRetrieveError, SeekId,
        matches::{Match, MatchRepository},
        seek::SeekService,
    },
    workflow::matchmaking::SeekView,
};

pub trait GetSeekUseCase {
    fn get_seek(&self, seek_id: SeekId) -> Option<SeekView>;
}

pub struct GetSeekUseCaseImpl<S: SeekService> {
    seek_service: Arc<S>,
}

impl<S: SeekService> GetSeekUseCaseImpl<S> {
    pub fn new(seek_service: Arc<S>) -> Self {
        Self { seek_service }
    }
}

impl<S: SeekService> GetSeekUseCase for GetSeekUseCaseImpl<S> {
    fn get_seek(&self, seek_id: SeekId) -> Option<SeekView> {
        self.seek_service.get_seek(seek_id).map(SeekView::from)
    }
}

#[async_trait::async_trait]
pub trait GetMatchUseCase {
    async fn get_match(&self, match_id: MatchId) -> Result<Match, GetMatchError>;
}

pub enum GetMatchError {
    NotFound,
    InternalError,
}

pub struct GetMatchUseCaseImpl<M: MatchRepository> {
    match_repo: Arc<M>,
}

impl<M: MatchRepository> GetMatchUseCaseImpl<M> {
    pub fn new(match_repo: Arc<M>) -> Self {
        Self { match_repo }
    }
}

#[async_trait::async_trait]
impl<M: MatchRepository + Send + Sync + 'static> GetMatchUseCase for GetMatchUseCaseImpl<M> {
    async fn get_match(&self, match_id: MatchId) -> Result<Match, GetMatchError> {
        match self.match_repo.get_match(match_id).await {
            Ok(m) => Ok(m),
            Err(RepoRetrieveError::NotFound) => Err(GetMatchError::NotFound),
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!("Failed to retrieve match {}: {:?}", match_id, e);
                Err(GetMatchError::InternalError)
            }
        }
    }
}
