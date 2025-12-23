use std::sync::Arc;

use crate::{
    app::{event::EventDispatcher, matchmaking::create_game_from_match},
    domain::{
        PlayerId, SeekId,
        game::GameService,
        game_history::{GameHistoryService, GameRepository},
        r#match::{MatchColorRule, MatchService},
        seek::{SeekEvent, SeekService},
    },
};

pub trait AcceptSeekUseCase {
    fn accept_seek(&self, player: PlayerId, seek_id: SeekId) -> Result<(), AcceptSeekError>;
}

pub struct AcceptSeekUseCaseImpl<
    S: SeekService,
    SD: EventDispatcher<SeekEvent>,
    G: GameService,
    GR: GameRepository,
    GH: GameHistoryService,
    M: MatchService,
> {
    seek_service: Arc<S>,
    seek_event_dispatcher: Arc<SD>,
    game_service: Arc<G>,
    game_repository: Arc<GR>,
    game_history_service: Arc<GH>,
    match_service: Arc<M>,
}

impl<
    S: SeekService,
    SD: EventDispatcher<SeekEvent>,
    G: GameService,
    GR: GameRepository,
    GH: GameHistoryService,
    M: MatchService,
> AcceptSeekUseCaseImpl<S, SD, G, GR, GH, M>
{
    pub fn new(
        seek_service: Arc<S>,
        seek_event_dispatcher: Arc<SD>,
        game_service: Arc<G>,
        game_repository: Arc<GR>,
        game_history_service: Arc<GH>,
        match_service: Arc<M>,
    ) -> Self {
        Self {
            seek_service,
            seek_event_dispatcher,
            game_service,
            game_repository,
            game_history_service,
            match_service,
        }
    }
}

pub enum AcceptSeekError {
    SeekNotFound,
    InvalidOpponent,
}

impl<
    S: SeekService,
    SD: EventDispatcher<SeekEvent>,
    G: GameService,
    GR: GameRepository,
    GH: GameHistoryService,
    M: MatchService,
> AcceptSeekUseCase for AcceptSeekUseCaseImpl<S, SD, G, GR, GH, M>
{
    fn accept_seek(&self, player: PlayerId, seek_id: SeekId) -> Result<(), AcceptSeekError> {
        let seek = self
            .seek_service
            .get_seek(seek_id)
            .ok_or(AcceptSeekError::SeekNotFound)?;

        if seek.opponent.is_some_and(|opp| opp != player) {
            return Err(AcceptSeekError::InvalidOpponent);
        }

        self.seek_service.cancel_seek(seek_id);

        if let Some(other_player_seek_id) = self.seek_service.get_seek_by_player(player) {
            self.seek_service.cancel_seek(other_player_seek_id);
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
            self.match_service.as_ref(),
            self.game_history_service.as_ref(),
            self.game_repository.as_ref(),
            self.game_service.as_ref(),
            &match_entry,
        );

        let events = self.seek_service.take_events();
        self.seek_event_dispatcher.handle_events(events);

        Ok(())
    }
}
