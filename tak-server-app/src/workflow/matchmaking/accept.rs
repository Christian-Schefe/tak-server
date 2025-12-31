use std::sync::Arc;

use crate::{
    domain::{
        PlayerId, SeekId,
        r#match::{MatchColorRule, MatchService},
        seek::SeekService,
    },
    ports::notification::{ListenerMessage, ListenerNotificationPort},
    workflow::matchmaking::create_game::CreateGameFromMatchWorkflow,
};

#[async_trait::async_trait]
pub trait AcceptSeekUseCase {
    async fn accept_seek(&self, player: PlayerId, seek_id: SeekId) -> Result<(), AcceptSeekError>;
}

pub struct AcceptSeekUseCaseImpl<
    S: SeekService,
    M: MatchService,
    L: ListenerNotificationPort,
    C: CreateGameFromMatchWorkflow,
> {
    seek_service: Arc<S>,
    match_service: Arc<M>,
    notification_port: Arc<L>,
    create_game_workflow: Arc<C>,
}

impl<S: SeekService, M: MatchService, L: ListenerNotificationPort, C: CreateGameFromMatchWorkflow>
    AcceptSeekUseCaseImpl<S, M, L, C>
{
    pub fn new(
        seek_service: Arc<S>,
        match_service: Arc<M>,
        notification_port: Arc<L>,
        create_game_workflow: Arc<C>,
    ) -> Self {
        Self {
            seek_service,
            match_service,
            notification_port,
            create_game_workflow,
        }
    }
}

pub enum AcceptSeekError {
    SeekNotFound,
    InvalidOpponent,
}

#[async_trait::async_trait]
impl<
    S: SeekService + Send + Sync + 'static,
    M: MatchService + Send + Sync + 'static,
    L: ListenerNotificationPort + Send + Sync + 'static,
    C: CreateGameFromMatchWorkflow + Send + Sync + 'static,
> AcceptSeekUseCase for AcceptSeekUseCaseImpl<S, M, L, C>
{
    async fn accept_seek(&self, player: PlayerId, seek_id: SeekId) -> Result<(), AcceptSeekError> {
        let seek = self
            .seek_service
            .cancel_seek(seek_id)
            .ok_or(AcceptSeekError::SeekNotFound)?;
        let message = ListenerMessage::SeekCanceled {
            seek: (&seek).into(),
        };
        self.notification_port.notify_all(message);

        if seek.opponent_id.is_some_and(|opp| opp != player) {
            return Err(AcceptSeekError::InvalidOpponent);
        }

        if let Some(other_player_seek_id) = self.seek_service.get_seek_by_player(player) {
            if let Some(cancelled_seek) = self.seek_service.cancel_seek(other_player_seek_id) {
                let message = ListenerMessage::SeekCanceled {
                    seek: cancelled_seek.into(),
                };
                self.notification_port.notify_all(message);
            }
        }

        let match_entry = self.match_service.create_match(
            seek.creator_id,
            player,
            seek.color,
            MatchColorRule::Alternate,
            seek.game_settings.clone(),
            seek.game_type,
        );

        self.create_game_workflow
            .create_game_from_match(&match_entry)
            .await;

        Ok(())
    }
}
