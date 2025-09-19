use std::sync::Arc;

use dashmap::DashMap;

use crate::{
    ArcClientService, ArcGameService, ServiceError, ServiceResult,
    player::PlayerUsername,
    protocol::ServerMessage,
    tak::{TakGameSettings, TakPlayer},
};

#[derive(Clone, Debug)]
pub struct Seek {
    pub id: SeekId,
    pub creator: PlayerUsername,
    pub opponent: Option<PlayerUsername>,
    pub color: Option<TakPlayer>,
    pub game_settings: TakGameSettings,
    pub game_type: GameType,
}

pub type SeekId = u32;

#[derive(Clone, Debug, PartialEq)]
pub enum GameType {
    Unrated,
    Rated,
    Tournament,
}

pub trait SeekService {
    fn add_seek(
        &self,
        player: PlayerUsername,
        opponent: Option<PlayerUsername>,
        color: Option<TakPlayer>,
        game_settings: TakGameSettings,
        game_type: GameType,
    ) -> ServiceResult<()>;
    fn get_seeks(&self) -> Vec<Seek>;
    fn remove_seek_of_player(&self, player: &PlayerUsername) -> ServiceResult<Seek>;
    fn accept_seek(&self, player: &PlayerUsername, id: &SeekId) -> ServiceResult<()>;
}

#[derive(Clone)]
pub struct SeekServiceImpl {
    client_service: ArcClientService,
    game_service: ArcGameService,
    seeks: Arc<DashMap<SeekId, Seek>>,
    seeks_by_player: Arc<DashMap<PlayerUsername, SeekId>>,
    next_seek_id: Arc<std::sync::Mutex<SeekId>>,
}

impl SeekServiceImpl {
    pub fn new(client_service: ArcClientService, game_service: ArcGameService) -> Self {
        Self {
            client_service,
            game_service,
            seeks: Arc::new(DashMap::new()),
            seeks_by_player: Arc::new(DashMap::new()),
            next_seek_id: Arc::new(std::sync::Mutex::new(1)),
        }
    }

    fn increment_seek_id(&self) -> SeekId {
        let mut id_lock = self
            .next_seek_id
            .lock()
            .expect("Failed to lock seek ID mutex");
        let seek_id = *id_lock;
        *id_lock += 1;
        seek_id
    }
}

impl SeekService for SeekServiceImpl {
    fn add_seek(
        &self,
        player: PlayerUsername,
        opponent: Option<PlayerUsername>,
        color: Option<TakPlayer>,
        game_settings: TakGameSettings,
        game_type: GameType,
    ) -> ServiceResult<()> {
        if !game_settings.is_valid() {
            return ServiceError::validation_err("Invalid game settings");
        }
        if self.seeks_by_player.contains_key(&player) {
            self.remove_seek_of_player(&player)?;
        }
        let seek_id = self.increment_seek_id();
        let seek = Seek {
            creator: player.clone(),
            id: seek_id,
            opponent,
            color,
            game_settings,
            game_type,
        };

        let seek_id = seek.id;
        self.seeks.insert(seek_id, seek.clone());
        self.seeks_by_player.insert(player, seek_id);

        let seek_new_msg = ServerMessage::SeekList {
            add: true,
            seek: seek.clone(),
        };
        self.client_service
            .try_auth_protocol_broadcast(&seek_new_msg);

        Ok(())
    }

    fn get_seeks(&self) -> Vec<Seek> {
        self.seeks
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    fn remove_seek_of_player(&self, player: &PlayerUsername) -> ServiceResult<Seek> {
        let Some((_, seek_id)) = self.seeks_by_player.remove(player) else {
            return ServiceError::not_found("No seek found for player");
        };
        let Some((_, seek)) = self.seeks.remove(&seek_id) else {
            return ServiceError::not_found("Seek ID not found");
        };

        let seek_remove_msg = ServerMessage::SeekList {
            add: false,
            seek: seek.clone(),
        };
        self.client_service
            .try_auth_protocol_broadcast(&seek_remove_msg);

        Ok(seek)
    }

    fn accept_seek(&self, player: &PlayerUsername, id: &SeekId) -> ServiceResult<()> {
        let Some(seek_ref) = self.seeks.get(id) else {
            return ServiceError::not_found("Seek ID not found");
        };
        let seek = seek_ref.value().clone();
        drop(seek_ref);
        if let Some(ref opponent) = seek.opponent {
            if opponent != player {
                return ServiceError::validation_err("This seek is not for you");
            }
        }
        self.game_service.add_game_from_seek(&seek, &player)?;
        let _ = self.remove_seek_of_player(&seek.creator);
        let _ = self.remove_seek_of_player(&player);
        Ok(())
    }
}
