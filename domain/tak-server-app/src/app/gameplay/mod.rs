use std::borrow::Borrow;

use tak_core::TakGame;

use crate::domain::{GameId, GameType, PlayerId, game::Game};

pub mod do_action;
pub mod get;
pub mod list;
pub mod observe;
pub mod offer_draw;
pub mod request_undo;
pub mod resign;

#[derive(Clone, Debug)]
pub struct GameView {
    pub id: GameId,
    pub white: PlayerId,
    pub black: PlayerId,
    pub game: TakGame,
    pub game_type: GameType,
}

impl GameView {
    fn from(id: GameId, game: impl Borrow<Game>) -> Self {
        let game = game.borrow();
        GameView {
            id,
            white: game.white,
            black: game.black,
            game: game.game.clone(),
            game_type: game.game_type,
        }
    }
}
