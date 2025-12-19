use std::sync::{Arc, Mutex};

use dashmap::DashMap;

use crate::domain::PlayerId;

pub trait PlayerService {
    fn set_player_online(&self, player_id: PlayerId);
    fn set_player_offline(&self, player_id: PlayerId);
    fn take_events(&self) -> Vec<PlayerEvent>;
}

pub enum PlayerEvent {
    PlayersOnline(Vec<PlayerId>),
}

pub struct PlayerServiceImpl {
    online_players: Arc<DashMap<PlayerId, ()>>,
    events: Arc<Mutex<Vec<PlayerEvent>>>,
}

impl PlayerServiceImpl {
    pub fn new() -> Self {
        Self {
            online_players: Arc::new(DashMap::new()),
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn add_event(&self, event: PlayerEvent) {
        let mut events = self.events.lock().unwrap();
        events.push(event);
    }

    fn update_players_online(&self) {
        let players: Vec<PlayerId> = self
            .online_players
            .iter()
            .map(|entry| *entry.key())
            .collect();
        self.add_event(PlayerEvent::PlayersOnline(players));
    }
}

impl PlayerService for PlayerServiceImpl {
    fn set_player_online(&self, player_id: PlayerId) {
        self.online_players.insert(player_id, ());
        self.update_players_online();
    }

    fn set_player_offline(&self, player_id: PlayerId) {
        self.online_players.remove(&player_id);
        self.update_players_online();
    }

    fn take_events(&self) -> Vec<PlayerEvent> {
        let mut events = self.events.lock().unwrap();
        std::mem::take(&mut *events)
    }
}
