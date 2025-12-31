use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use tak_core::{TakGameSettings, TakPlayer};

use crate::domain::{GameType, PlayerId, SeekId};

#[derive(Clone, Debug, PartialEq)]
pub struct Seek {
    pub id: SeekId,
    pub creator_id: PlayerId,
    pub opponent_id: Option<PlayerId>,
    pub color: Option<TakPlayer>,
    pub game_settings: TakGameSettings,
    pub game_type: GameType,
}

pub enum CreateSeekError {
    InvalidGameSettings,
    InvalidOpponent,
}

pub trait SeekService {
    fn create_seek(
        &self,
        player: PlayerId,
        opponent: Option<PlayerId>,
        color: Option<TakPlayer>,
        game_settings: TakGameSettings,
        game_type: GameType,
    ) -> Result<Seek, CreateSeekError>;
    fn get_seek_by_player(&self, player: PlayerId) -> Option<SeekId>;
    fn get_seek(&self, seek_id: SeekId) -> Option<Seek>;
    fn list_seeks(&self) -> Vec<Seek>;
    fn cancel_seek(&self, seek_id: SeekId) -> Option<Seek>;
}

#[derive(Clone)]
pub struct SeekServiceImpl {
    seeks: Arc<DashMap<SeekId, Seek>>,
    seeks_by_player: Arc<DashMap<PlayerId, SeekId>>,
    next_seek_id: Arc<Mutex<SeekId>>,
}

impl SeekServiceImpl {
    pub fn new() -> Self {
        Self {
            seeks: Arc::new(DashMap::new()),
            seeks_by_player: Arc::new(DashMap::new()),
            next_seek_id: Arc::new(Mutex::new(SeekId(0))),
        }
    }

    fn increment_seek_id(&self) -> SeekId {
        let mut id_lock = self.next_seek_id.lock().unwrap();
        let seek_id = *id_lock;
        id_lock.0 += 1;
        seek_id
    }
}

impl SeekService for SeekServiceImpl {
    fn create_seek(
        &self,
        player: PlayerId,
        opponent: Option<PlayerId>,
        color: Option<TakPlayer>,
        game_settings: TakGameSettings,
        game_type: GameType,
    ) -> Result<Seek, CreateSeekError> {
        if !game_settings.is_valid() {
            return Err(CreateSeekError::InvalidGameSettings);
        }
        if opponent.is_some_and(|opp| opp == player) {
            return Err(CreateSeekError::InvalidOpponent);
        }
        let seek_id = self.increment_seek_id();
        let seek = Seek {
            id: seek_id,
            creator_id: player,
            opponent_id: opponent,
            color,
            game_settings,
            game_type,
        };
        self.seeks.insert(seek_id, seek.clone());
        self.seeks_by_player.insert(player, seek_id);

        Ok(seek)
    }

    fn get_seek_by_player(&self, player: PlayerId) -> Option<SeekId> {
        self.seeks_by_player.get(&player).as_deref().cloned()
    }

    fn get_seek(&self, seek_id: SeekId) -> Option<Seek> {
        self.seeks.get(&seek_id).map(|entry| entry.value().clone())
    }

    fn list_seeks(&self) -> Vec<Seek> {
        self.seeks
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    fn cancel_seek(&self, seek_id: SeekId) -> Option<Seek> {
        if let Some((_, seek)) = self.seeks.remove(&seek_id) {
            self.seeks_by_player.remove(&seek.creator_id);
            Some(seek)
        } else {
            None
        }
    }
}
