use std::borrow::Borrow;

use tak_core::{TakGame, TakGameSettings};

use crate::domain::{GameId, GameType, MatchId, PlayerId, game::Game};

pub mod do_action;
pub mod finalize_game;
pub mod get;
pub mod list;
pub mod observe;
pub mod timeout;

#[derive(Clone, Debug)]
pub struct GameView {
    pub id: GameId,
    pub match_id: MatchId,
    pub white: PlayerId,
    pub black: PlayerId,
    pub game: TakGame,
    pub game_type: GameType,
    pub settings: TakGameSettings,
}

impl GameView {
    fn from(game: impl Borrow<Game>) -> Self {
        let game = game.borrow();
        GameView {
            id: game.game_id,
            match_id: game.match_id,
            white: game.white,
            black: game.black,
            game: game.game.clone(),
            game_type: game.game_type,
            settings: game.settings.clone(),
        }
    }
}
