use std::sync::Arc;

use crate::{
    domain::{
        PlayerId, SeekId,
        game::GameService,
        game_history::{GameHistoryService, GameRepository},
        r#match::{MatchColorRule, MatchService},
        seek::SeekService,
    },
    ports::notification::{ListenerMessage, ListenerNotificationPort},
    processes::game_timeout_runner::GameTimeoutRunner,
    workflow::matchmaking::create_game_from_match,
};

pub trait AcceptSeekUseCase {
    fn accept_seek(&self, player: PlayerId, seek_id: SeekId) -> Result<(), AcceptSeekError>;
}

pub struct AcceptSeekUseCaseImpl<
    S: SeekService,
    G: GameService,
    GR: GameRepository,
    GH: GameHistoryService,
    M: MatchService,
    L: ListenerNotificationPort,
    GT: GameTimeoutRunner,
> {
    seek_service: Arc<S>,
    game_service: Arc<G>,
    game_repository: Arc<GR>,
    game_history_service: Arc<GH>,
    match_service: Arc<M>,
    notification_port: Arc<L>,
    game_timeout_scheduler: Arc<GT>,
}

impl<
    S: SeekService,
    G: GameService,
    GR: GameRepository,
    GH: GameHistoryService,
    M: MatchService,
    L: ListenerNotificationPort,
    GT: GameTimeoutRunner,
> AcceptSeekUseCaseImpl<S, G, GR, GH, M, L, GT>
{
    pub fn new(
        seek_service: Arc<S>,
        game_service: Arc<G>,
        game_repository: Arc<GR>,
        game_history_service: Arc<GH>,
        match_service: Arc<M>,
        notification_port: Arc<L>,
        game_timeout_scheduler: Arc<GT>,
    ) -> Self {
        Self {
            seek_service,
            game_service,
            game_repository,
            game_history_service,
            match_service,
            notification_port,
            game_timeout_scheduler,
        }
    }
}

pub enum AcceptSeekError {
    SeekNotFound,
    InvalidOpponent,
}

impl<
    S: SeekService,
    G: GameService,
    GR: GameRepository,
    GH: GameHistoryService,
    M: MatchService,
    L: ListenerNotificationPort,
    GT: GameTimeoutRunner,
> AcceptSeekUseCase for AcceptSeekUseCaseImpl<S, G, GR, GH, M, L, GT>
{
    fn accept_seek(&self, player: PlayerId, seek_id: SeekId) -> Result<(), AcceptSeekError> {
        let seek = self
            .seek_service
            .cancel_seek(seek_id)
            .ok_or(AcceptSeekError::SeekNotFound)?;
        let message = ListenerMessage::SeekCanceled {
            seek: (&seek).into(),
        };
        self.notification_port.notify_all(message);

        if seek.opponent.is_some_and(|opp| opp != player) {
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
            seek.creator,
            player,
            seek.color,
            MatchColorRule::Alternate,
            seek.game_settings.clone(),
            seek.game_type,
        );

        create_game_from_match(
            &self.match_service,
            &self.game_history_service,
            &self.game_repository,
            &self.game_service,
            &self.game_timeout_scheduler,
            &match_entry,
        );

        Ok(())
    }
}
