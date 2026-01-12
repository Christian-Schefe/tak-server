use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};
use tak_core::{TakGameSettings, TakPlayer};

use crate::domain::{PlayerId, SeekId};

#[derive(Clone, Debug, PartialEq)]
pub struct Seek {
    pub id: SeekId,
    pub creator_id: PlayerId,
    pub opponent_id: Option<PlayerId>,
    pub color: Option<TakPlayer>,
    pub game_settings: TakGameSettings,
    pub is_rated: bool,
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
        is_rated: bool,
    ) -> Result<Seek, CreateSeekError>;
    fn cancel_all_player_seeks(&self, player: PlayerId) -> Vec<Seek>;
    fn get_seek(&self, seek_id: SeekId) -> Option<Seek>;
    fn list_seeks(&self) -> Vec<Seek>;
    fn remove_seek(&self, seek_id: SeekId) -> Option<Seek>;
}

struct SeekRegistry {
    seeks: HashMap<SeekId, Seek>,
    seeks_by_player: HashMap<PlayerId, HashSet<SeekId>>,
    next_seek_id: SeekId,
}

impl SeekRegistry {
    fn new() -> Self {
        Self {
            seeks: HashMap::new(),
            seeks_by_player: HashMap::new(),
            next_seek_id: SeekId(0),
        }
    }

    fn increment_seek_id(&mut self) -> SeekId {
        let seek_id = self.next_seek_id.clone();
        self.next_seek_id.0 += 1;
        seek_id
    }

    fn add_seek(&mut self, make_seek: impl FnOnce(SeekId) -> Seek) -> Seek {
        let seek_id = self.increment_seek_id();
        let seek = make_seek(seek_id);
        self.seeks.insert(seek_id, seek.clone());
        self.seeks_by_player
            .entry(seek.creator_id)
            .or_default()
            .insert(seek_id);
        seek
    }

    fn cancel_all_player_seeks(&mut self, player: PlayerId) -> Vec<Seek> {
        let mut canceled_seeks = Vec::new();
        if let Some(seek_ids) = self.seeks_by_player.remove(&player) {
            for seek_id in seek_ids {
                if let Some(seek) = self.seeks.remove(&seek_id) {
                    canceled_seeks.push(seek);
                }
            }
        }
        canceled_seeks
    }

    fn get_seek(&self, seek_id: SeekId) -> Option<&Seek> {
        self.seeks.get(&seek_id)
    }

    fn get_seeks(&self) -> impl Iterator<Item = &Seek> {
        self.seeks.values()
    }

    fn remove_seek(&mut self, seek_id: SeekId) -> Option<Seek> {
        if let Some(seek) = self.seeks.remove(&seek_id) {
            self.seeks_by_player
                .get_mut(&seek.creator_id)
                .map(|seek_ids| seek_ids.remove(&seek_id));
            Some(seek)
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct SeekServiceImpl {
    seek_registry: Arc<RwLock<SeekRegistry>>,
}

impl SeekServiceImpl {
    pub fn new() -> Self {
        Self {
            seek_registry: Arc::new(RwLock::new(SeekRegistry::new())),
        }
    }
}

impl SeekService for SeekServiceImpl {
    fn create_seek(
        &self,
        player: PlayerId,
        opponent: Option<PlayerId>,
        color: Option<TakPlayer>,
        game_settings: TakGameSettings,
        is_rated: bool,
    ) -> Result<Seek, CreateSeekError> {
        if !game_settings.is_valid() {
            return Err(CreateSeekError::InvalidGameSettings);
        }
        if opponent.is_some_and(|opp| opp == player) {
            return Err(CreateSeekError::InvalidOpponent);
        }
        Ok(self
            .seek_registry
            .write()
            .unwrap()
            .add_seek(|seek_id| Seek {
                id: seek_id,
                creator_id: player,
                opponent_id: opponent,
                color,
                game_settings,
                is_rated,
            }))
    }

    fn cancel_all_player_seeks(&self, player: PlayerId) -> Vec<Seek> {
        self.seek_registry
            .write()
            .unwrap()
            .cancel_all_player_seeks(player)
    }

    fn get_seek(&self, seek_id: SeekId) -> Option<Seek> {
        self.seek_registry
            .read()
            .unwrap()
            .get_seek(seek_id)
            .cloned()
    }

    fn list_seeks(&self) -> Vec<Seek> {
        self.seek_registry
            .read()
            .unwrap()
            .get_seeks()
            .cloned()
            .collect()
    }

    fn remove_seek(&self, seek_id: SeekId) -> Option<Seek> {
        self.seek_registry.write().unwrap().remove_seek(seek_id)
    }
}
