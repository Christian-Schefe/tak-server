use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use uuid::Uuid;

use crate::domain::{AccountId, PlayerId};

pub struct Player {
    pub player_id: PlayerId,
    pub is_silenced: bool,
    pub is_banned: bool,
    pub is_bot: bool,
}

impl Player {
    pub fn new() -> Self {
        Self {
            player_id: PlayerId(Uuid::new_v4()),
            is_silenced: false,
            is_banned: false,
            is_bot: false,
        }
    }
}

pub trait PlayerRepository {
    fn create_player(&self, player: Player);
    fn delete_player(&self, player_id: PlayerId);
    fn get_player(&self, player_id: PlayerId) -> Option<Player>;
    fn link_account(&self, player_id: PlayerId, account_id: AccountId);
    fn unlink_account(&self, player_id: PlayerId);
    fn get_player_by_account_id(&self, account_id: AccountId) -> Option<Player>;
    fn get_account_for_player(&self, player_id: PlayerId) -> Option<AccountId>;
    fn set_player_silenced(&self, player_id: PlayerId, silenced: bool);
    fn set_player_banned(&self, player_id: PlayerId, banned: bool);
    fn set_player_is_bot(&self, player_id: PlayerId, is_bot: bool);
}

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
