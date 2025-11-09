use std::{
    fmt::Display,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use dashmap::DashMap;
use tak_core::{TakActionRecord, TakGameState};

use crate::{
    app::LazyAppState,
    game::{Game, GameId},
    player::PlayerUsername,
    seek::{Seek, SeekId},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ListenerId(uuid::Uuid);

impl Display for ListenerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ListenerId {
    pub fn new() -> Self {
        ListenerId(uuid::Uuid::new_v4())
    }
}

pub type ArcTransportService = Arc<Box<dyn TransportService + Send + Sync + 'static>>;

#[async_trait::async_trait]
pub trait TransportService {
    async fn disconnect_listener(&self, id: ListenerId, reason: DisconnectReason);
    async fn try_listener_send(&self, id: ListenerId, msg: &ServerMessage);
    async fn try_listener_multicast(&self, ids: &[ListenerId], msg: &ServerMessage) {
        let futures = ids.iter().map(|id| self.try_listener_send(*id, msg));
        futures::future::join_all(futures).await;
    }
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
    ClientQuit,
    NewSession,
    Inactivity,
    Kick,
    Ban(String),
    ServerShutdown,
}

#[derive(Clone, Debug)]
pub enum ServerGameMessage {
    Action {
        action: TakActionRecord,
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
    pub sent_messages: Arc<Mutex<Vec<(ListenerId, ServerMessage)>>>,
}

#[allow(unused)]
impl MockTransportService {
    pub fn get_messages(&self) -> Vec<(ListenerId, ServerMessage)> {
        self.sent_messages.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl TransportService for MockTransportService {
    async fn disconnect_listener(&self, _id: ListenerId, _reason: DisconnectReason) {
        //No-op
    }
    async fn try_listener_send(&self, id: ListenerId, msg: &ServerMessage) {
        self.sent_messages
            .lock()
            .unwrap()
            .push((id.clone(), msg.clone()));
    }
}

pub struct CompositeTransportService {
    services: Vec<ArcTransportService>,
}

impl CompositeTransportService {
    pub fn new(services: Vec<ArcTransportService>) -> Self {
        Self { services }
    }
}

#[async_trait::async_trait]
impl TransportService for CompositeTransportService {
    async fn disconnect_listener(&self, id: ListenerId, reason: DisconnectReason) {
        let futures = self
            .services
            .iter()
            .map(|service| service.disconnect_listener(id.clone(), reason.clone()));
        futures::future::join_all(futures).await;
    }
    async fn try_listener_send(&self, id: ListenerId, msg: &ServerMessage) {
        let futures = self
            .services
            .iter()
            .map(|service| service.try_listener_send(id.clone(), msg));
        futures::future::join_all(futures).await;
    }
}

pub type ArcPlayerConnectionService = Arc<Box<dyn PlayerConnectionService + Send + Sync>>;

#[async_trait::async_trait]
pub trait PlayerConnectionService {
    async fn on_player_connected(&self, listener_id: ListenerId, username: &PlayerUsername);
    async fn on_player_disconnected(&self, listener_id: ListenerId, username: &PlayerUsername);
    fn on_listener_connected(&self, listener_id: ListenerId);
    fn on_listener_disconnected(&self, listener_id: ListenerId);
    //Implementations are required to hold activity status information for longer than the game disconnect timeout
    fn get_last_connected(&self, username: &PlayerUsername) -> Option<Instant>;
    fn get_connected_players(&self) -> Vec<(PlayerUsername, ListenerId)>;
    fn get_player_connection(&self, username: &PlayerUsername) -> Option<ListenerId>;
}

pub struct PlayerConnectionServiceImpl {
    online_players: Arc<DashMap<PlayerUsername, ListenerId>>,
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

    async fn update_online_players(&self) {
        let players = self.get_connected_players();
        let (usernames, ids): (Vec<_>, Vec<_>) = players.into_iter().unzip();

        let msg = ServerMessage::PlayersOnline { players: usernames };
        let transport = self.app_state.transport_service();

        let mut futures = Vec::new();
        for id in &ids {
            let fut = transport.try_listener_send(*id, &msg);
            futures.push(fut);
        }
        futures::future::join_all(futures).await;
    }
}

#[async_trait::async_trait]
impl PlayerConnectionService for PlayerConnectionServiceImpl {
    fn get_player_connection(&self, username: &PlayerUsername) -> Option<ListenerId> {
        self.online_players
            .get(username)
            .map(|entry| entry.value().clone())
    }

    fn get_connected_players(&self) -> Vec<(PlayerUsername, ListenerId)> {
        self.online_players
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    async fn on_player_connected(&self, listener_id: ListenerId, username: &PlayerUsername) {
        self.online_players
            .insert(username.clone(), listener_id.clone());
        self.update_online_players().await;
    }

    async fn on_player_disconnected(&self, _listener_id: ListenerId, username: &PlayerUsername) {
        let _ = self
            .app_state
            .seek_service()
            .remove_seek_of_player(username);

        self.online_players.remove(username);
        let now = Instant::now();
        let _ = self.last_connected_cache.insert(username.clone(), now);
        self.update_online_players().await;
    }

    fn on_listener_connected(&self, _listener_id: ListenerId) {
        //No-op
    }

    fn on_listener_disconnected(&self, listener_id: ListenerId) {
        let _ = self
            .app_state
            .chat_service()
            .leave_all_rooms_quiet(listener_id);
        let _ = self.app_state.game_service().unobserve_all(listener_id);
    }

    fn get_last_connected(&self, username: &PlayerUsername) -> Option<Instant> {
        if self.online_players.contains_key(username) {
            return Some(Instant::now());
        }
        self.last_connected_cache.get(username)
    }
}

#[derive(Clone)]
pub struct MockPlayerConnectionService {
    connected_players: Arc<Mutex<Vec<(PlayerUsername, ListenerId)>>>,
}

impl MockPlayerConnectionService {
    pub fn new() -> Self {
        Self {
            connected_players: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait::async_trait]
impl PlayerConnectionService for MockPlayerConnectionService {
    async fn on_player_connected(&self, listener_id: ListenerId, username: &PlayerUsername) {
        self.connected_players
            .lock()
            .unwrap()
            .push((username.clone(), listener_id.clone()));
    }

    async fn on_player_disconnected(&self, _listener_id: ListenerId, username: &PlayerUsername) {
        let mut players = self.connected_players.lock().unwrap();
        players.retain(|(u, _)| u != username);
    }

    fn on_listener_connected(&self, _listener_id: ListenerId) {
        //No-op
    }

    fn on_listener_disconnected(&self, _listener_id: ListenerId) {
        //No-op
    }

    fn get_last_connected(&self, username: &PlayerUsername) -> Option<Instant> {
        let players = self.connected_players.lock().unwrap();
        for (u, _) in players.iter() {
            if u == username {
                return Some(Instant::now());
            }
        }
        None
    }

    fn get_connected_players(&self) -> Vec<(PlayerUsername, ListenerId)> {
        self.connected_players.lock().unwrap().clone()
    }

    fn get_player_connection(&self, username: &PlayerUsername) -> Option<ListenerId> {
        let players = self.connected_players.lock().unwrap();
        for (u, id) in players.iter() {
            if u == username {
                return Some(id.clone());
            }
        }
        None
    }
}

pub async fn do_player_broadcast(
    player_connection_service: &ArcPlayerConnectionService,
    transport_service: &ArcTransportService,
    msg: &ServerMessage,
) {
    let players = player_connection_service.get_connected_players();
    let futures = players
        .into_iter()
        .map(|(_username, listener_id)| transport_service.try_listener_send(listener_id, msg));
    futures::future::join_all(futures).await;
}

pub async fn do_player_send(
    player_connection_service: &ArcPlayerConnectionService,
    transport_service: &ArcTransportService,
    username: &PlayerUsername,
    msg: &ServerMessage,
) {
    if let Some(listener_id) = player_connection_service.get_player_connection(username) {
        transport_service.try_listener_send(listener_id, msg).await;
    }
}
