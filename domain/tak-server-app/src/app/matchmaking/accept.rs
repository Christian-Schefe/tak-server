use std::sync::Arc;

use crate::{
    app::event::EventDispatcher,
    domain::{
        PlayerId, SeekId,
        game::GameService,
        game_history::{GameHistoryService, GameRepository},
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
> {
    seek_service: Arc<S>,
    seek_event_dispatcher: Arc<SD>,
    game_service: Arc<G>,
    game_repository: Arc<GR>,
    game_history_service: Arc<GH>,
}

impl<
    S: SeekService,
    SD: EventDispatcher<SeekEvent>,
    G: GameService,
    GR: GameRepository,
    GH: GameHistoryService,
> AcceptSeekUseCaseImpl<S, SD, G, GR, GH>
{
    pub fn new(
        seek_service: Arc<S>,
        seek_event_dispatcher: Arc<SD>,
        game_service: Arc<G>,
        game_repository: Arc<GR>,
        game_history_service: Arc<GH>,
    ) -> Self {
        Self {
            seek_service,
            seek_event_dispatcher,
            game_service,
            game_repository,
            game_history_service,
        }
    }
}

pub enum AcceptSeekError {
    SeekNotFound,
    InvalidOpponent,
    InvalidSeek,
}

impl<
    S: SeekService,
    SD: EventDispatcher<SeekEvent>,
    G: GameService,
    GR: GameRepository,
    GH: GameHistoryService,
> AcceptSeekUseCase for AcceptSeekUseCaseImpl<S, SD, G, GR, GH>
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

        let (game_id, game) = match self.game_service.create_game(
            seek.creator,
            player,
            seek.color,
            seek.game_type,
            seek.game_settings.clone(),
        ) {
            Ok(res) => res,
            Err(_) => return Err(AcceptSeekError::InvalidSeek),
        };

        let game_record = self.game_history_service.get_ongoing_game_record(
            game.white,
            game.black,
            seek.game_settings,
            seek.game_type,
        );
        let finished_game_id = self.game_repository.save_ongoing_game(game_record);

        self.game_history_service
            .save_ongoing_game_id(game_id, finished_game_id);

        let events = self.seek_service.take_events();
        self.seek_event_dispatcher.handle_events(events);

        Ok(())
    }
}
