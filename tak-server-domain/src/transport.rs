use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use dashmap::DashMap;
use tak_core::{TakAction, TakGameState};

use crate::{
    chat::ArcChatService,
    game::{ArcGameService, Game, GameId, SpectatorId},
    player::PlayerUsername,
    seek::{ArcSeekService, Seek, SeekId},
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
    //Implementations are required to hold activity status information for longer than the game disconnect timeout
    fn get_last_active(&self, username: &PlayerUsername) -> Option<ActivityStatus>;
}

pub enum ActivityStatus {
    Active,
    InactiveSince(std::time::Instant),
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

    fn get_last_active(&self, _username: &PlayerUsername) -> Option<ActivityStatus> {
        Some(ActivityStatus::Active)
    }
}

pub type ArcPlayerConnectionService = Arc<Box<dyn PlayerConnectionService + Send + Sync>>;
pub trait PlayerConnectionService {
    fn on_player_connected(&self, username: &PlayerUsername);
    fn on_player_disconnected(&self, username: &PlayerUsername);
    fn on_spectator_connected(&self, spectator_id: &SpectatorId);
    fn on_spectator_disconnected(&self, spectator_id: &SpectatorId);
}

pub struct PlayerConnectionServiceImpl {
    online_players: Arc<DashMap<PlayerUsername, ()>>,
    seek_service: ArcSeekService,
    game_service: ArcGameService,
    chat_service: ArcChatService,
    transport_service: ArcTransportService,
}

impl PlayerConnectionServiceImpl {
    pub fn new(
        seek_service: ArcSeekService,
        game_service: ArcGameService,
        chat_service: ArcChatService,
        transport_service: ArcTransportService,
    ) -> Self {
        Self {
            online_players: Arc::new(DashMap::new()),
            seek_service,
            game_service,
            chat_service,
            transport_service,
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

        self.transport_service.try_player_broadcast(&msg);
    }
}

impl PlayerConnectionService for PlayerConnectionServiceImpl {
    fn on_player_connected(&self, username: &PlayerUsername) {
        self.online_players.insert(username.clone(), ());
        self.update_online_players();
    }

    fn on_player_disconnected(&self, username: &PlayerUsername) {
        let _ = self.seek_service.remove_seek_of_player(username);

        self.online_players.remove(username);
        self.update_online_players();
    }

    fn on_spectator_connected(&self, _spectator_id: &SpectatorId) {
        //No-op
    }

    fn on_spectator_disconnected(&self, spectator_id: &SpectatorId) {
        let _ = self.chat_service.leave_all_rooms(spectator_id);
        let _ = self.game_service.unobserve_all(spectator_id);
    }
}
