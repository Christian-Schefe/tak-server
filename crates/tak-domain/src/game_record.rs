use std::time::SystemTime;

use tak_core::{TakGameSettings, TakPlayer};

use crate::{MatchId, UserId, game::Game};

#[derive(Debug, Clone)]
pub struct PlayerSnapshot {
    pub user_id: UserId,
    pub username: String,
    pub rating: Option<f64>,
}

impl PlayerSnapshot {
    pub fn new(user_id: UserId, username: String, rating: Option<f64>) -> Self {
        Self {
            user_id,
            username,
            rating,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameMetadata {
    pub date: chrono::DateTime<chrono::Utc>,
    pub white: PlayerSnapshot,
    pub black: PlayerSnapshot,
    pub settings: TakGameSettings,
    pub is_rated: bool,
    pub match_id: Option<MatchId>,
}

impl GameMetadata {
    pub fn new(
        date: chrono::DateTime<chrono::Utc>,
        white: PlayerSnapshot,
        black: PlayerSnapshot,
        is_rated: bool,
        settings: TakGameSettings,
        match_id: Option<MatchId>,
    ) -> Self {
        Self {
            date,
            white,
            black,
            settings,
            is_rated,
            match_id,
        }
    }
    pub fn get_opponent(&self, player: UserId) -> Option<UserId> {
        if player == self.white.user_id {
            Some(self.black.user_id)
        } else if player == self.black.user_id {
            Some(self.white.user_id)
        } else {
            None
        }
    }
    pub fn get_user_id(&self, player: TakPlayer) -> UserId {
        match player {
            TakPlayer::White => self.white.user_id,
            TakPlayer::Black => self.black.user_id,
        }
    }

    pub fn get_player(&self, id: UserId) -> Option<TakPlayer> {
        if id == self.white.user_id {
            Some(TakPlayer::White)
        } else if id == self.black.user_id {
            Some(TakPlayer::Black)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameRatingInfo {
    pub rating_change_white: f64,
    pub rating_change_black: f64,
}

#[derive(Debug, Clone)]
pub struct GameRecord {
    pub metadata: GameMetadata,
    pub rating_info: Option<GameRatingInfo>,
    pub game: Game,
}

impl GameRecord {
    pub fn new_ongoing(metadata: GameMetadata, now: SystemTime) -> Self {
        let game = Game::new(&metadata.settings, now);
        Self {
            metadata,
            game,
            rating_info: None,
        }
    }
}
