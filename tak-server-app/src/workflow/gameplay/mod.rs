use std::borrow::Borrow;

use tak_core::{TakFinishedRealtimeGame, TakOngoingRealtimeGame, TakRealtimeGameSettings};

use crate::domain::{
    GameId, PlayerId,
    game::{FinishedGame, GameMetadata, OngoingGame, request::GameRequest},
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
    pub is_rated: bool,
    pub settings: TakRealtimeGameSettings,
}

#[derive(Clone, Debug)]
pub struct OngoingGameView {
    pub metadata: GameMetadataView,
    pub game: TakOngoingRealtimeGame,
    pub requests: Vec<GameRequest>,
}

#[derive(Clone, Debug)]
pub struct FinishedGameView {
    pub metadata: GameMetadataView,
    pub game: TakFinishedRealtimeGame,
}

impl GameMetadataView {
    pub fn from(game: impl Borrow<GameMetadata>) -> Self {
        let game = game.borrow();
        GameMetadataView {
            id: game.game_id,
            white_id: game.white_id,
            black_id: game.black_id,
            is_rated: game.is_rated,
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
            requests: game.requests.get_all_requests(),
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
