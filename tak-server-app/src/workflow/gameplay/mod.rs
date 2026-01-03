use std::borrow::Borrow;

use tak_core::{TakGame, TakGameSettings};

use crate::domain::{GameId, GameType, PlayerId, game::Game};

pub mod do_action;
pub mod finalize_game;
pub mod get;
pub mod list;
pub mod observe;
pub mod timeout;

#[derive(Clone, Debug)]
pub struct GameView {
    pub id: GameId,
    pub white_id: PlayerId,
    pub black_id: PlayerId,
    pub game: TakGame,
    pub game_type: GameType,
    pub settings: TakGameSettings,
}

impl GameView {
    pub fn from(game: impl Borrow<Game>) -> Self {
        let game = game.borrow();
        GameView {
            id: game.game_id,
            white_id: game.white_id,
            black_id: game.black_id,
            game: game.game.clone(),
            game_type: game.game_type,
            settings: game.settings.clone(),
        }
    }
}
