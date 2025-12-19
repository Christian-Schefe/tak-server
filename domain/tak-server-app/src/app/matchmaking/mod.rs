use std::borrow::Borrow;

use tak_core::{TakGameSettings, TakPlayer};

use crate::domain::{GameId, GameType, PlayerId, SeekId, seek::Seek};

pub mod accept;
pub mod cancel;
pub mod create;
pub mod get;
pub mod list;
pub mod notify_seek;

#[derive(Debug)]
pub struct SeekView {
    pub id: SeekId,
    pub creator: PlayerId,
    pub opponent: Option<PlayerId>,
    pub color: Option<TakPlayer>,
    pub game_settings: TakGameSettings,
    pub game_type: GameType,
    pub rematch_from: Option<GameId>,
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
            rematch_from: seek.rematch_from,
        }
    }
}
