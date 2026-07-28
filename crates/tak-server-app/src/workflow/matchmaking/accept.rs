use std::sync::Arc;

use tak_core::{TakPlayer, TakTimeSettings};

use crate::{
    domain::{
        PlayerId, SeekId,
        matches::{Match, MatchMode, MatchRepository, MatchSettings},
        seek::SeekService,
    },
    ports::notification::{ListenerMessage, ListenerNotificationPort},
    workflow::matchmaking::create_game::{CreateGameFromMatchError, CreateGameFromMatchWorkflow},
};

#[async_trait::async_trait]
pub trait AcceptSeekUseCase {
    async fn accept_seek(&self, player: PlayerId, seek_id: SeekId) -> Result<(), AcceptSeekError>;
}

pub struct AcceptSeekUseCaseImpl<
    S: SeekService,
    M: MatchRepository,
    L: ListenerNotificationPort,
    C: CreateGameFromMatchWorkflow,
> {
    seek_service: Arc<S>,
    match_repo: Arc<M>,
    notification_port: Arc<L>,
    create_game_workflow: Arc<C>,
}

impl<
    S: SeekService,
    M: MatchRepository,
    L: ListenerNotificationPort,
    C: CreateGameFromMatchWorkflow,
> AcceptSeekUseCaseImpl<S, M, L, C>
{
    pub fn new(
        seek_service: Arc<S>,
        match_repo: Arc<M>,
        notification_port: Arc<L>,
        create_game_workflow: Arc<C>,
    ) -> Self {
        Self {
            seek_service,
            match_repo,
            notification_port,
            create_game_workflow,
        }
    }
}

pub enum AcceptSeekError {
    SeekNotFound,
    FailedToCreateGame,
}

#[async_trait::async_trait]
impl<
    S: SeekService + Send + Sync + 'static,
    M: MatchRepository + Send + Sync + 'static,
    L: ListenerNotificationPort + Send + Sync + 'static,
    C: CreateGameFromMatchWorkflow + Send + Sync + 'static,
> AcceptSeekUseCase for AcceptSeekUseCaseImpl<S, M, L, C>
{
    #[tracing::instrument(skip(self))]
    async fn accept_seek(&self, player: PlayerId, seek_id: SeekId) -> Result<(), AcceptSeekError> {
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

        let match_id = match self.match_repo.create_match(match_data).await {
            Ok(id) => id,
            Err(e) => {
                tracing::error!(
                    "Failed to create match from accepted seek {}: {}",
                    seek_id,
                    e
                );
                return Err(AcceptSeekError::FailedToCreateGame);
            }
        };

        match self
            .create_game_workflow
            .create_game_from_match(match_id)
            .await
        {
            Ok(_) => Ok(()),
            Err(CreateGameFromMatchError::AlreadyInProgress) => {
                tracing::error!(
                    "Failed to create game from match {}: already in progress",
                    match_id
                );
                Err(AcceptSeekError::FailedToCreateGame)
            }
            Err(CreateGameFromMatchError::RepositoryError) => {
                tracing::error!(
                    "Failed to create game from match {}: repository error",
                    match_id
                );
                Err(AcceptSeekError::FailedToCreateGame)
            }
            Err(CreateGameFromMatchError::MatchNotFound) => {
                tracing::error!(
                    "Failed to create game from match {}: match not found",
                    match_id
                );
                Err(AcceptSeekError::FailedToCreateGame)
            }
        }
    }
}
