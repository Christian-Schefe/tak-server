use tak_core::{TakGameSettings, TakPlayer};

use crate::UserId;

#[derive(Clone, Debug, PartialEq)]
pub struct Seek {
    pub creator_id: UserId,
    pub color: Option<TakPlayer>,
    pub game_settings: TakGameSettings,
    pub is_rated: bool,
}

impl Seek {
    pub fn new(
        player: UserId,
        color: Option<TakPlayer>,
        game_settings: TakGameSettings,
        is_rated: bool,
    ) -> Option<Self> {
        if !game_settings.is_valid() {
            return None;
        }
        Some(Self {
            creator_id: player,
            color,
            game_settings,
            is_rated,
        })
    }
}
