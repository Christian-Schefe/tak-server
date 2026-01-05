use std::borrow::Borrow;

use tak_core::{TakFinishedGame, TakGameSettings, TakOngoingGame};

use crate::domain::{
    GameId, GameType, PlayerId,
    game::{FinishedGame, GameMetadata, OngoingGame},
};

pub mod do_action;
pub mod finalize_game;
pub mod get;
pub mod list;
pub mod observe;
pub mod timeout;

#[derive(Clone, Debug)]
pub struct GameMetadataView {
    pub id: GameId,
    pub white_id: PlayerId,
    pub black_id: PlayerId,
    pub game_type: GameType,
    pub settings: TakGameSettings,
}

#[derive(Clone, Debug)]
pub struct OngoingGameView {
    pub metadata: GameMetadataView,
    pub game: TakOngoingGame,
}

#[derive(Clone, Debug)]
pub struct FinishedGameView {
    pub metadata: GameMetadataView,
    pub game: TakFinishedGame,
}

impl GameMetadataView {
    pub fn from(game: impl Borrow<GameMetadata>) -> Self {
        let game = game.borrow();
        GameMetadataView {
            id: game.game_id,
            white_id: game.white_id,
            black_id: game.black_id,
            game_type: game.game_type,
            settings: game.settings.clone(),
        }
    }
}

impl OngoingGameView {
    pub fn from(game: impl Borrow<OngoingGame>) -> Self {
        let game = game.borrow();
        OngoingGameView {
            metadata: GameMetadataView::from(&game.metadata),
            game: game.game.clone(),
        }
    }
}

impl FinishedGameView {
    pub fn from(game: impl Borrow<FinishedGame>) -> Self {
        let game = game.borrow();
        FinishedGameView {
            metadata: GameMetadataView::from(&game.metadata),
            game: game.game.clone(),
        }
    }
}
