use std::borrow::Borrow;

use tak_core::TakGame;

use crate::app::domain::{GameId, GameType, PlayerId, game::Game};

pub mod do_action;
pub mod finalize_game;
pub mod get;
pub mod list;
pub mod observe;

#[derive(Clone, Debug)]
pub struct GameView {
    pub id: GameId,
    pub white: PlayerId,
    pub black: PlayerId,
    pub game: TakGame,
    pub game_type: GameType,
}

impl GameView {
    fn from(game: impl Borrow<Game>) -> Self {
        let game = game.borrow();
        GameView {
            id: game.game_id,
            white: game.white,
            black: game.black,
            game: game.game.clone(),
            game_type: game.game_type,
        }
    }
}
