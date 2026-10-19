use std::sync::Arc;

use crate::{
    PortsCollection,
    ports::{
        matches::MatchRepository,
        transaction::{TransactionError, TransactionManager},
    },
    workflow::matchmaking::SeekView,
};
use tak_domain::{MatchId, SeekId, matches::Match, seek::SeekService};

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

pub struct GetMatchUseCaseImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection> GetMatchUseCaseImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static> GetMatchUseCase
    for GetMatchUseCaseImpl<Ports>
{
    async fn get_match(&self, match_id: MatchId) -> Result<Match, GetMatchError> {
        match self
            .ports
            .transaction_manager()
            .execute(self.ports.match_repository().get_match(match_id))
            .await
        {
            Ok(Some(m)) => Ok(m),
            Ok(None) => Err(GetMatchError::NotFound),
            Err(TransactionError(e)) => {
                tracing::error!("Failed to retrieve match {}: {:?}", match_id, e);
                Err(GetMatchError::InternalError)
            }
        }
    }
}
