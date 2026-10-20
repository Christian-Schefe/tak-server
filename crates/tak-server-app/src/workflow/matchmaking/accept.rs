use std::sync::Arc;

use crate::{
    PortsCollection,
    ports::{
        matches::MatchRepository,
        notification::{ListenerMessage, ListenerNotificationPort},
        transaction::{RepoError, TransactionManager},
    },
    workflow::matchmaking::create_game::{CreateGameFromMatchError, CreateGameFromMatchWorkflow},
};
use tak_core::{TakPlayer, TakTimeSettings};
use tak_domain::{
    SeekId, UserId,
    matches::{Match, MatchMode, MatchSettings},
    seek::SeekService,
};

#[async_trait::async_trait]
pub trait AcceptSeekUseCase {
    async fn accept_seek(&self, player: UserId, seek_id: SeekId) -> Result<(), AcceptSeekError>;
}

pub struct AcceptSeekUseCaseImpl<
    Ports: PortsCollection,
    S: SeekService,
    L: ListenerNotificationPort,
    C: CreateGameFromMatchWorkflow<Ports::TrCtx>,
> {
    ports: Arc<Ports>,
    seek_service: Arc<S>,
    notification_port: Arc<L>,
    create_game_workflow: Arc<C>,
}

impl<
    Ports: PortsCollection,
    S: SeekService,
    L: ListenerNotificationPort,
    C: CreateGameFromMatchWorkflow<Ports::TrCtx>,
> AcceptSeekUseCaseImpl<Ports, S, L, C>
{
    pub fn new(
        ports: Arc<Ports>,
        seek_service: Arc<S>,
        notification_port: Arc<L>,
        create_game_workflow: Arc<C>,
    ) -> Self {
        Self {
            ports,
            seek_service,
            notification_port,
            create_game_workflow,
        }
    }
}

pub enum AcceptSeekError {
    SeekNotFound,
    Internal,
}

#[async_trait::async_trait]
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    S: SeekService + Send + Sync + 'static,
    L: ListenerNotificationPort + Send + Sync + 'static,
    C: CreateGameFromMatchWorkflow<Ports::TrCtx> + Send + Sync + 'static,
> AcceptSeekUseCase for AcceptSeekUseCaseImpl<Ports, S, L, C>
{
    #[tracing::instrument(skip(self))]
    async fn accept_seek(&self, player: UserId, seek_id: SeekId) -> Result<(), AcceptSeekError> {
        let seek = self
            .seek_service
            .remove_seek_if(seek_id, |s| s.creator_id != player)
            .ok_or(AcceptSeekError::SeekNotFound)?;

        let cancelled_seeks = self.seek_service.cancel_player_seeks(player, |seek| {
            matches!(
                seek.game_settings.time_settings,
                TakTimeSettings::Realtime(_)
            )
        });
        let cancelled_seeks2 = self
            .seek_service
            .cancel_player_seeks(seek.creator_id, |seek| {
                matches!(
                    seek.game_settings.time_settings,
                    TakTimeSettings::Realtime(_)
                )
            });

        for cancelled_seek in cancelled_seeks
            .into_iter()
            .chain(cancelled_seeks2.into_iter())
        {
            let message = ListenerMessage::SeekCancelled {
                seek: (&cancelled_seek).into(),
            };
            self.notification_port.notify_all(&message);
        }

        let message = ListenerMessage::SeekAccepted {
            seek: (&seek).into(),
        };
        self.notification_port.notify_all(&message);

        let initial_color = seek.color.unwrap_or_else(|| {
            if rand::random::<bool>() {
                TakPlayer::White
            } else {
                TakPlayer::Black
            }
        });

        let match_settings = MatchSettings {
            game_settings: seek.game_settings.clone(),
            match_mode: MatchMode::Unlimited,
            is_rated: seek.is_rated,
        };

        let match_data = Match::new(
            seek.creator_id,
            player,
            None,
            match_settings.clone(),
            initial_color,
        );

        let ctx = self
            .ports
            .transaction_manager()
            .transaction()
            .await
            .map_err(|RepoError(e)| {
                tracing::error!(
                    "Failed to start transaction for accepting seek {}: {}",
                    seek_id,
                    e
                );
                AcceptSeekError::Internal
            })?;
        let match_id = match self
            .ports
            .match_repository()
            .create_match(&ctx, match_data)
            .await
        {
            Ok(id) => id,
            Err(RepoError(e)) => {
                tracing::error!(
                    "Failed to create match from accepted seek {}: {}",
                    seek_id,
                    e
                );
                return Err(AcceptSeekError::Internal);
            }
        };

        match self
            .create_game_workflow
            .create_game_from_match(&ctx, match_id)
            .await
        {
            Ok(_) => Ok(()),
            Err(CreateGameFromMatchError::AlreadyInProgress) => {
                tracing::error!(
                    "Failed to create game from match {}: already in progress",
                    match_id
                );
                Err(AcceptSeekError::Internal)
            }
            Err(CreateGameFromMatchError::Internal) => {
                tracing::error!(
                    "Failed to create game from match {}: internal error",
                    match_id
                );
                Err(AcceptSeekError::Internal)
            }
            Err(CreateGameFromMatchError::MatchNotFound) => {
                tracing::error!(
                    "Failed to create game from match {}: match not found",
                    match_id
                );
                Err(AcceptSeekError::Internal)
            }
        }?;

        self.ports
            .transaction_manager()
            .commit(ctx)
            .await
            .map_err(|RepoError(e)| {
                tracing::error!(
                    "Failed to commit transaction for accepting seek {}: {}",
                    seek_id,
                    e
                );
                AcceptSeekError::Internal
            })?;
        Ok(())
    }
}
