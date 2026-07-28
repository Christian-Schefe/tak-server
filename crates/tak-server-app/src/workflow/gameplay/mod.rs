use std::borrow::Borrow;

use chrono::{DateTime, Utc};
use tak_core::{TakFinishedGame, TakGameSettings, TakOngoingGame};

use crate::domain::{
    GameId, MatchId, PlayerId,
    game::{FinishedGame, GameMetadata, OngoingGame, request::GameRequests},
};

pub mod do_action;
pub mod finalize_game;
pub mod get;
pub mod list;
pub mod observe;
pub mod timeout;

#[derive(Clone, Debug)]
pub struct GameMetadataView {
    pub date: DateTime<Utc>,
    pub white_id: PlayerId,
    pub black_id: PlayerId,
    pub is_rated: bool,
    pub settings: TakGameSettings,
    pub match_id: Option<MatchId>,
}

#[derive(Clone, Debug)]
pub struct OngoingGameView {
    pub id: GameId,
    pub metadata: GameMetadataView,
    pub game: TakOngoingGame,
    pub white_requests: GameRequests,
    pub black_requests: GameRequests,
}

#[derive(Clone, Debug)]
pub struct FinishedGameView {
    pub id: GameId,
    pub metadata: GameMetadataView,
    pub game: TakFinishedGame,
}

impl GameMetadataView {
    pub fn from(game: impl Borrow<GameMetadata>) -> Self {
        let game = game.borrow();
        GameMetadataView {
            date: game.date,
            white_id: game.white_id,
            black_id: game.black_id,
            is_rated: game.is_rated,
            settings: game.settings.clone(),
            match_id: game.match_id,
        }
    }
}

impl OngoingGameView {
    pub fn from(game: impl Borrow<OngoingGame>) -> Self {
        let game = game.borrow();
        OngoingGameView {
            id: game.game_id,
            metadata: GameMetadataView::from(&game.metadata),
            game: game.game.clone(),
            white_requests: game.requests.white_requests.clone(),
            black_requests: game.requests.black_requests.clone(),
        }
    }
}

impl FinishedGameView {
    pub fn from(game: impl Borrow<FinishedGame>) -> Self {
        let game = game.borrow();
        FinishedGameView {
            id: game.game_id,
            metadata: GameMetadataView::from(&game.metadata),
            game: game.game.clone(),
        }
    }
}
