use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use dashmap::DashMap;
use tak_core::{TakAction, TakGameState};

use crate::{
    app::LazyAppState,
    game::{Game, GameId, SpectatorId},
    player::PlayerUsername,
    seek::{Seek, SeekId},
};

pub type ArcTransportService = Arc<Box<dyn TransportService + Send + Sync + 'static>>;
pub trait TransportService {
    fn try_player_send(&self, id: &PlayerUsername, msg: &ServerMessage);
    fn try_spectator_send(&self, id: &SpectatorId, msg: &ServerMessage);
    fn try_player_multicast(&self, ids: &[PlayerUsername], msg: &ServerMessage) {
        for id in ids {
            self.try_player_send(id, msg);
        }
    }
    fn try_spectator_multicast(&self, ids: &[SpectatorId], msg: &ServerMessage) {
        for id in ids {
            self.try_spectator_send(id, msg);
        }
    }
    fn try_player_broadcast(&self, msg: &ServerMessage);
}

#[derive(Clone, Debug)]
pub enum ServerMessage {
    SeekList {
        add: bool,
        seek: Seek,
    },
    GameList {
        add: bool,
        game: Game,
    },
    GameStart {
        game_id: GameId,
    },
    GameMessage {
        game_id: GameId,
        message: ServerGameMessage,
    },
    PlayersOnline {
        players: Vec<String>,
    },
    ChatMessage {
        from: PlayerUsername,
        message: String,
        source: ChatMessageSource,
    },
    RoomMembership {
        room: String,
        joined: bool,
    },
    AcceptRematch {
        seek_id: SeekId,
    },
    ConnectionClosed {
        reason: DisconnectReason,
    },
}

#[derive(Clone, Debug)]
pub enum DisconnectReason {
    NewSession,
    Inactivity,
    Kick,
    Ban(String),
}

#[derive(Clone, Debug)]
pub enum ServerGameMessage {
    Action {
        action: TakAction,
    },
    TimeUpdate {
        remaining_white: Duration,
        remaining_black: Duration,
    },
    Undo,
    GameOver {
        game_state: TakGameState,
    },
    UndoRequest {
        request: bool,
    },
    DrawOffer {
        offer: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChatMessageSource {
    Global,
    Room { name: String },
    Private,
}

#[derive(Clone, Default)]
pub struct MockTransportService {
    pub sent_messages: Arc<Mutex<Vec<(PlayerUsername, ServerMessage)>>>,
    pub sent_spectator_messages: Arc<Mutex<Vec<(SpectatorId, ServerMessage)>>>,
    pub sent_broadcasts: Arc<Mutex<Vec<ServerMessage>>>,
}

#[allow(unused)]
impl MockTransportService {
    pub fn get_messages(&self) -> Vec<(PlayerUsername, ServerMessage)> {
        self.sent_messages.lock().unwrap().clone()
    }

    pub fn get_spectator_messages(&self) -> Vec<(SpectatorId, ServerMessage)> {
        self.sent_spectator_messages.lock().unwrap().clone()
    }

    pub fn get_broadcasts(&self) -> Vec<ServerMessage> {
        self.sent_broadcasts.lock().unwrap().clone()
    }
}

impl TransportService for MockTransportService {
    fn try_player_send(&self, id: &PlayerUsername, msg: &ServerMessage) {
        self.sent_messages
            .lock()
            .unwrap()
            .push((id.clone(), msg.clone()));
    }

    fn try_spectator_send(&self, id: &SpectatorId, msg: &ServerMessage) {
        self.sent_spectator_messages
            .lock()
            .unwrap()
            .push((id.clone(), msg.clone()));
    }

    fn try_player_broadcast(&self, msg: &ServerMessage) {
        self.sent_broadcasts.lock().unwrap().push(msg.clone());
    }
}

pub type ArcPlayerConnectionService = Arc<Box<dyn PlayerConnectionService + Send + Sync>>;
pub trait PlayerConnectionService {
    fn on_player_connected(&self, username: &PlayerUsername);
    fn on_player_disconnected(&self, username: &PlayerUsername);
    fn on_spectator_connected(&self, spectator_id: &SpectatorId);
    fn on_spectator_disconnected(&self, spectator_id: &SpectatorId);
    //Implementations are required to hold activity status information for longer than the game disconnect timeout
    fn get_last_connected(&self, username: &PlayerUsername) -> Option<Instant>;
}

pub struct PlayerConnectionServiceImpl {
    online_players: Arc<DashMap<PlayerUsername, ()>>,
    last_connected_cache: Arc<moka::sync::Cache<PlayerUsername, Instant>>,
    app_state: LazyAppState,
}

impl PlayerConnectionServiceImpl {
    pub fn new(app_state: LazyAppState) -> Self {
        Self {
            online_players: Arc::new(DashMap::new()),
            last_connected_cache: Arc::new(
                moka::sync::Cache::builder()
                    .time_to_live(Duration::from_secs(3600))
                    .build(),
            ),
            app_state,
        }
    }
}

impl PlayerConnectionServiceImpl {
    fn update_online_players(&self) {
        let players: Vec<String> = self
            .online_players
            .iter()
            .map(|entry| entry.key().to_string())
            .collect();

        let msg = ServerMessage::PlayersOnline { players };

        self.app_state
            .transport_service()
            .try_player_broadcast(&msg);
    }
}

impl PlayerConnectionService for PlayerConnectionServiceImpl {
    fn on_player_connected(&self, username: &PlayerUsername) {
        self.online_players.insert(username.clone(), ());
        self.update_online_players();
    }

    fn on_player_disconnected(&self, username: &PlayerUsername) {
        let _ = self
            .app_state
            .seek_service()
            .remove_seek_of_player(username);

        self.online_players.remove(username);
        let now = Instant::now();
        let _ = self.last_connected_cache.insert(username.clone(), now);
        self.update_online_players();
    }

    fn on_spectator_connected(&self, _spectator_id: &SpectatorId) {
        //No-op
    }

    fn on_spectator_disconnected(&self, spectator_id: &SpectatorId) {
        let _ = self.app_state.chat_service().leave_all_rooms(spectator_id);
        let _ = self.app_state.game_service().unobserve_all(spectator_id);
    }

    fn get_last_connected(&self, username: &PlayerUsername) -> Option<Instant> {
        if self.online_players.contains_key(username) {
            return Some(Instant::now());
        }
        self.last_connected_cache.get(username)
    }
}
