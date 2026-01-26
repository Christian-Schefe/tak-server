use std::borrow::Borrow;

use tak_core::{TakGameSettings, TakPlayer};

use crate::domain::{PlayerId, SeekId, seek::Seek};

pub mod accept;
pub mod cancel;
pub mod cleanup;
pub mod create;
pub mod create_game;
pub mod get;
pub mod list;
pub mod rematch;

#[derive(Clone, Debug)]
pub struct SeekView {
    pub id: SeekId,
    pub creator_id: PlayerId,
    pub opponent_id: Option<PlayerId>,
    pub color: Option<TakPlayer>,
    pub game_settings: TakGameSettings,
    pub is_rated: bool,
}

impl<T: Borrow<Seek>> From<T> for SeekView {
    fn from(seek: T) -> Self {
        let seek = seek.borrow();
        SeekView {
            id: seek.id,
            creator_id: seek.creator_id,
            opponent_id: seek.opponent_id,
            color: seek.color,
            game_settings: seek.game_settings.clone(),
            is_rated: seek.is_rated,
        }
    }
}
