use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

use tak_core::TakTimeSettings;
use tak_domain::{SeekId, UserId, seek::Seek};

pub trait SeekService {
    fn create_seek(&self, seek: Seek) -> SeekId;
    fn cancel_player_seeks(&self, player: UserId, only_realtime: bool) -> Vec<(SeekId, Seek)>;
    fn cancel_seek(&self, player: UserId, seek_id: SeekId) -> Option<Seek>;
    fn get_seek(&self, seek_id: SeekId) -> Option<Seek>;
    fn list_seeks(&self) -> Vec<(SeekId, Seek)>;
    fn try_accept_seek(&self, seek_id: SeekId, user_id: UserId) -> Option<Seek>;
}

struct SeekRegistry {
    seeks: HashMap<SeekId, Seek>,
    seeks_by_player: HashMap<UserId, HashSet<SeekId>>,
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

    fn add_seek(&mut self, seek: Seek) -> SeekId {
        let seek_id = self.increment_seek_id();
        self.seeks.insert(seek_id, seek.clone());
        self.seeks_by_player
            .entry(seek.creator_id)
            .or_default()
            .insert(seek_id);
        seek_id
    }

    fn cancel_player_seeks(&mut self, player: UserId, only_realtime: bool) -> Vec<(SeekId, Seek)> {
        let mut cancelled_seeks = Vec::new();
        if let Some(seek_ids) = self.seeks_by_player.remove(&player) {
            for seek_id in seek_ids {
                if let Some(seek) = self.seeks.get(&seek_id)
                    && (!only_realtime
                        || matches!(
                            seek.game_settings.time_settings,
                            TakTimeSettings::Realtime(_)
                        ))
                {
                    cancelled_seeks.push((seek_id, seek.clone()));
                    self.seeks.remove(&seek_id);
                }
            }
        }
        cancelled_seeks
    }

    fn cancel_player_seek(&mut self, player: UserId, seek_id: SeekId) -> Option<Seek> {
        if let Some(seek) = self.seeks.get(&seek_id) {
            if seek.creator_id == player {
                if let Some(seek_ids) = self.seeks_by_player.get_mut(&player) {
                    seek_ids.remove(&seek_id);
                    if seek_ids.is_empty() {
                        self.seeks_by_player.remove(&player);
                    }
                }
                return self.seeks.remove(&seek_id);
            }
        }
        None
    }

    fn get_seek(&self, seek_id: SeekId) -> Option<&Seek> {
        self.seeks.get(&seek_id)
    }

    fn get_seeks(&self) -> impl Iterator<Item = (SeekId, &Seek)> {
        self.seeks.iter().map(|(id, seek)| (*id, seek))
    }

    fn remove_seek_if(
        &mut self,
        seek_id: SeekId,
        predicate: impl Fn(&Seek) -> bool,
    ) -> Option<Seek> {
        if let Some(seek) = self.seeks.get(&seek_id).filter(|s| predicate(s)) {
            let seek = seek.clone();
            self.seeks.remove(&seek_id);
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
    fn create_seek(&self, seek: Seek) -> SeekId {
        self.seek_registry.write().unwrap().add_seek(seek)
    }

    fn cancel_player_seeks(&self, player: UserId, only_realtime: bool) -> Vec<(SeekId, Seek)> {
        self.seek_registry
            .write()
            .unwrap()
            .cancel_player_seeks(player, only_realtime)
    }

    fn cancel_seek(&self, player: UserId, seek_id: SeekId) -> Option<Seek> {
        self.seek_registry
            .write()
            .unwrap()
            .cancel_player_seek(player, seek_id)
    }

    fn get_seek(&self, seek_id: SeekId) -> Option<Seek> {
        self.seek_registry
            .read()
            .unwrap()
            .get_seek(seek_id)
            .cloned()
    }

    fn list_seeks(&self) -> Vec<(SeekId, Seek)> {
        self.seek_registry
            .read()
            .unwrap()
            .get_seeks()
            .map(|(id, seek)| (id, seek.clone()))
            .collect()
    }

    fn try_accept_seek(&self, seek_id: SeekId, user_id: UserId) -> Option<Seek> {
        self.seek_registry
            .write()
            .unwrap()
            .remove_seek_if(seek_id, |seek| seek.creator_id != user_id)
    }
}
