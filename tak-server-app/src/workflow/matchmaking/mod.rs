use std::borrow::Borrow;

use tak_core::{TakGameSettings, TakPlayer};

use crate::domain::{GameType, PlayerId, SeekId, seek::Seek};

pub mod accept;
pub mod cancel;
pub mod cleanup;
pub mod create;
pub mod create_game;
pub mod get;
pub mod list;
pub mod rematch;

#[derive(Debug)]
pub struct SeekView {
    pub id: SeekId,
    pub creator: PlayerId,
    pub opponent: Option<PlayerId>,
    pub color: Option<TakPlayer>,
    pub game_settings: TakGameSettings,
    pub game_type: GameType,
}

impl<T: Borrow<Seek>> From<T> for SeekView {
    fn from(seek: T) -> Self {
        let seek = seek.borrow();
        SeekView {
            id: seek.id,
            creator: seek.creator,
            opponent: seek.opponent,
            color: seek.color,
            game_settings: seek.game_settings.clone(),
            game_type: seek.game_type,
        }
    }
}
