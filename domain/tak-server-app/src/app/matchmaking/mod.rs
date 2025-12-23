use std::borrow::Borrow;

use tak_core::{TakGameSettings, TakPlayer};

use crate::domain::{
    GameType, PlayerId, SeekId,
    game::GameService,
    game_history::{GameHistoryService, GameRepository},
    r#match::{Match, MatchService},
    seek::Seek,
};

pub mod accept;
pub mod cancel;
pub mod cleanup;
pub mod create;
pub mod get;
pub mod list;
pub mod notify_seek;
pub mod rematch;

#[derive(Debug)]
pub struct SeekView {
    pub id: SeekId,
    pub creator: PlayerId,
    pub opponent: Option<PlayerId>,
    pub color: Option<TakPlayer>,
    pub game_settings: TakGameSettings,
    pub game_type: GameType,
}

impl<T: Borrow<Seek>> From<T> for SeekView {
    fn from(seek: T) -> Self {
        let seek = seek.borrow();
        SeekView {
            id: seek.id,
            creator: seek.creator,
            opponent: seek.opponent,
            color: seek.color,
            game_settings: seek.game_settings.clone(),
            game_type: seek.game_type,
        }
    }
}

fn create_game_from_match<
    M: MatchService,
    GH: GameHistoryService,
    GR: GameRepository,
    G: GameService,
>(
    match_service: &M,
    game_history_service: &GH,
    game_repository: &GR,
    game_service: &G,
    match_entry: &Match,
) {
    let game = game_service.create_game(
        match_entry.player1,
        match_entry.player2,
        match_entry.inital_color,
        match_entry.game_type,
        match_entry.game_settings.clone(),
        match_entry.id,
    );
    match_service.start_game_in_match(match_entry.id, game.game_id);

    let game_record = game_history_service.get_ongoing_game_record(
        game.white,
        game.black,
        game.settings.clone(),
        game.game_type,
    );
    let finished_game_id = game_repository.save_ongoing_game(game_record);

    game_history_service.save_ongoing_game_id(game.game_id, finished_game_id);
}
