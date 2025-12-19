use std::sync::Arc;

use crate::{
    app::event::EventDispatcher,
    domain::{
        PlayerId, SeekId,
        game::GameService,
        seek::{SeekEvent, SeekService},
    },
};

pub trait AcceptSeekUseCase {
    fn accept_seek(&self, player: PlayerId, seek_id: SeekId) -> Result<(), AcceptSeekError>;
}

pub struct AcceptSeekUseCaseImpl<S: SeekService, SD: EventDispatcher<SeekEvent>, G: GameService> {
    seek_service: Arc<S>,
    seek_event_dispatcher: Arc<SD>,
    game_service: Arc<G>,
}

impl<S: SeekService, SD: EventDispatcher<SeekEvent>, G: GameService>
    AcceptSeekUseCaseImpl<S, SD, G>
{
    pub fn new(seek_service: Arc<S>, seek_event_dispatcher: Arc<SD>, game_service: Arc<G>) -> Self {
        Self {
            seek_service,
            seek_event_dispatcher,
            game_service,
        }
    }
}

pub enum AcceptSeekError {
    SeekNotFound,
    InvalidOpponent,
    InvalidSeek,
}

impl<S: SeekService, SD: EventDispatcher<SeekEvent>, G: GameService> AcceptSeekUseCase
    for AcceptSeekUseCaseImpl<S, SD, G>
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

        match self.game_service.create_game(
            seek.creator,
            player,
            seek.color,
            seek.game_type,
            seek.game_settings,
        ) {
            Ok(_) => {}
            Err(_) => return Err(AcceptSeekError::InvalidSeek),
        }

        let events = self.seek_service.take_events();
        self.seek_event_dispatcher.handle_events(events);

        Ok(())
    }
}
