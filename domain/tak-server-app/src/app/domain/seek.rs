use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use tak_core::{TakGameSettings, TakPlayer};

use crate::app::domain::{GameType, PlayerId, SeekId};

#[derive(Clone, Debug, PartialEq)]
pub struct Seek {
    pub id: SeekId,
    pub creator: PlayerId,
    pub opponent: Option<PlayerId>,
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
    ) -> Result<SeekId, CreateSeekError>;
    fn get_seek_by_player(&self, player: PlayerId) -> Option<SeekId>;
    fn get_seek(&self, seek_id: SeekId) -> Option<Seek>;
    fn list_seeks(&self) -> Vec<Seek>;
    fn cancel_seek(&self, seek_id: SeekId) -> Option<Seek>;
    fn take_events(&self) -> Vec<SeekEvent>;
}

pub enum SeekEvent {
    Created(Seek),
    Canceled(Seek),
}

#[derive(Clone)]
pub struct SeekServiceImpl {
    seeks: Arc<DashMap<SeekId, Seek>>,
    seeks_by_player: Arc<DashMap<PlayerId, SeekId>>,
    next_seek_id: Arc<Mutex<SeekId>>,
    events: Arc<Mutex<Vec<SeekEvent>>>,
}

impl SeekServiceImpl {
    pub fn new() -> Self {
        Self {
            seeks: Arc::new(DashMap::new()),
            seeks_by_player: Arc::new(DashMap::new()),
            next_seek_id: Arc::new(Mutex::new(SeekId(0))),
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn increment_seek_id(&self) -> SeekId {
        let mut id_lock = self.next_seek_id.lock().unwrap();
        let seek_id = *id_lock;
        id_lock.0 += 1;
        seek_id
    }

    fn add_event(&self, event: SeekEvent) {
        let mut events_lock = self.events.lock().unwrap();
        events_lock.push(event);
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
    ) -> Result<SeekId, CreateSeekError> {
        if !game_settings.is_valid() {
            return Err(CreateSeekError::InvalidGameSettings);
        }
        if opponent.is_some_and(|opp| opp == player) {
            return Err(CreateSeekError::InvalidOpponent);
        }
        let seek_id = self.increment_seek_id();
        let seek = Seek {
            id: seek_id,
            creator: player,
            opponent,
            color,
            game_settings,
            game_type,
        };
        self.seeks.insert(seek_id, seek.clone());
        self.seeks_by_player.insert(player, seek_id);

        self.add_event(SeekEvent::Created(seek));
        Ok(seek_id)
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
            self.seeks_by_player.remove(&seek.creator);
            self.add_event(SeekEvent::Canceled(seek.clone()));
            Some(seek)
        } else {
            None
        }
    }

    fn take_events(&self) -> Vec<SeekEvent> {
        let mut events_lock = self.events.lock().unwrap();
        std::mem::take(&mut *events_lock)
    }
}
