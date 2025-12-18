use std::borrow::Borrow;

use tak_core::TakGame;

use crate::domain::{GameId, GameType, PlayerId, game::Game};

mod do_move;
mod get;
mod list;
mod observe;
mod request_draw;
mod request_undo;
mod resign;

#[derive(Clone, Debug)]
pub struct GameView {
    pub id: GameId,
    pub white: PlayerId,
    pub black: PlayerId,
    pub game: TakGame,
    pub game_type: GameType,
}

impl<T: Borrow<Game>> From<T> for GameView {
    fn from(game: T) -> Self {
        let game = game.borrow();
        GameView {
            id: game.id,
            white: game.white,
            black: game.black,
            game: game.game.clone(),
            game_type: game.game_type,
        }
    }
}
