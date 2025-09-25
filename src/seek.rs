use std::sync::Arc;

use dashmap::DashMap;

use crate::{
    ArcClientService, ArcGameService, ServiceError, ServiceResult,
    game::GameId,
    player::PlayerUsername,
    protocol::ServerMessage,
    tak::{TakGameSettings, TakPlayer},
};

#[derive(Clone, Debug, PartialEq)]
pub struct Seek {
    pub id: SeekId,
    pub creator: PlayerUsername,
    pub opponent: Option<PlayerUsername>,
    pub color: Option<TakPlayer>,
    pub game_settings: TakGameSettings,
    pub game_type: GameType,
    pub rematch_from: Option<GameId>,
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
    ) -> ServiceResult<SeekId>;
    fn add_rematch_seek(
        &self,
        player: PlayerUsername,
        opponent: PlayerUsername,
        color: Option<TakPlayer>,
        game_settings: TakGameSettings,
        game_type: GameType,
        from_game: GameId,
    ) -> ServiceResult<()>;
    fn get_seek_ids(&self) -> Vec<SeekId>;
    fn get_seeks(&self) -> Vec<Seek>;
    fn get_seek(&self, id: &SeekId) -> ServiceResult<Seek>;
    fn remove_seek_of_player(&self, player: &PlayerUsername) -> ServiceResult<Seek>;
    fn accept_seek(&self, player: &PlayerUsername, id: &SeekId) -> ServiceResult<()>;
}

#[derive(Clone)]
pub struct SeekServiceImpl {
    client_service: ArcClientService,
    game_service: ArcGameService,
    seeks: Arc<DashMap<SeekId, Seek>>,
    seeks_by_player: Arc<DashMap<PlayerUsername, SeekId>>,
    rematch_seeks: Arc<DashMap<GameId, SeekId>>,
    next_seek_id: Arc<std::sync::Mutex<SeekId>>,
}

impl SeekServiceImpl {
    pub fn new(client_service: ArcClientService, game_service: ArcGameService) -> Self {
        Self {
            client_service,
            game_service,
            seeks: Arc::new(DashMap::new()),
            seeks_by_player: Arc::new(DashMap::new()),
            rematch_seeks: Arc::new(DashMap::new()),
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

    fn add_seek_internal(
        &self,
        player: PlayerUsername,
        opponent: Option<PlayerUsername>,
        color: Option<TakPlayer>,
        game_settings: TakGameSettings,
        game_type: GameType,
        from_game: Option<GameId>,
    ) -> ServiceResult<SeekId> {
        if !game_settings.is_valid() {
            return ServiceError::bad_request("Invalid game settings");
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
            rematch_from: from_game,
        };

        let seek_id = seek.id;
        self.seeks.insert(seek_id, seek.clone());
        self.seeks_by_player.insert(player, seek_id);

        println!("New seek: {:?}", seek);
        let seek_new_msg = ServerMessage::SeekList { add: true, seek };
        self.client_service
            .try_auth_protocol_broadcast(&seek_new_msg);

        Ok(seek_id)
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
    ) -> ServiceResult<SeekId> {
        self.add_seek_internal(player, opponent, color, game_settings, game_type, None)
    }

    fn add_rematch_seek(
        &self,
        player: PlayerUsername,
        opponent: PlayerUsername,
        color: Option<TakPlayer>,
        game_settings: TakGameSettings,
        game_type: GameType,
        from_game: GameId,
    ) -> ServiceResult<()> {
        // rematch seek entry is removed when the rematch seek gets accepted, so no need to remove it here
        if let Some(existing_seek_id) = self.rematch_seeks.get(&from_game) {
            let seek = self
                .seeks
                .get(&existing_seek_id)
                .ok_or_else(|| ServiceError::NotFound("Seek ID not found".to_string()))?;
            if seek.creator != opponent {
                return ServiceError::not_possible("This rematch seek is not for you");
            }
            drop(seek);
            let accept_rematch_msg = ServerMessage::AcceptRematch {
                seek_id: *existing_seek_id,
            };
            drop(existing_seek_id);
            self.client_service
                .get_associated_client(&player)
                .map(|id| {
                    self.client_service
                        .try_protocol_send(&id, &accept_rematch_msg);
                });
            return Ok(());
        }
        let seek_id = self.add_seek_internal(
            player,
            Some(opponent),
            color,
            game_settings,
            game_type,
            Some(from_game),
        )?;
        self.rematch_seeks.insert(from_game, seek_id);

        Ok(())
    }

    fn get_seek(&self, id: &SeekId) -> ServiceResult<Seek> {
        let Some(seek_ref) = self.seeks.get(id) else {
            return ServiceError::not_found("Seek ID not found");
        };
        Ok(seek_ref.value().clone())
    }

    fn get_seek_ids(&self) -> Vec<SeekId> {
        self.seeks.iter().map(|entry| entry.key().clone()).collect()
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

        if let Some(from_game) = seek.rematch_from {
            self.rematch_seeks.remove(&from_game);
        }

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
                return ServiceError::bad_request("This seek is not for you");
            }
        }
        self.game_service.add_game_from_seek(&seek, &player)?;
        let _ = self.remove_seek_of_player(&seek.creator);
        let _ = self.remove_seek_of_player(&player);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use uuid::Uuid;

    use crate::{
        client::{ClientService, MockClientService},
        game::MockGameService,
        tak::TakTimeControl,
    };

    use super::*;

    #[test]
    fn test_add_seek() {
        let mock_client_service = MockClientService::default();
        let mock_game_service = MockGameService::default();
        let seek_service = SeekServiceImpl::new(
            Arc::new(Box::new(mock_client_service.clone())),
            Arc::new(Box::new(mock_game_service.clone())),
        );

        let game_settings = TakGameSettings {
            board_size: 5,
            half_komi: 0,
            reserve_pieces: 21,
            reserve_capstones: 1,
            time_control: TakTimeControl {
                contingent: Duration::from_secs(300),
                increment: Duration::from_secs(5),
                extra: None,
            },
        };

        let expected_seek = Seek {
            id: 1,
            creator: "player1".to_string(),
            opponent: None,
            color: None,
            game_settings: game_settings.clone(),
            game_type: GameType::Rated,
            rematch_from: None,
        };

        let invalid_game_settings = TakGameSettings {
            board_size: 0,
            half_komi: 0,
            reserve_pieces: 0,
            reserve_capstones: 0,
            time_control: TakTimeControl {
                contingent: Duration::from_secs(0),
                increment: Duration::from_secs(0),
                extra: None,
            },
        };

        assert!(
            seek_service
                .add_seek(
                    "player1".to_string(),
                    None,
                    None,
                    invalid_game_settings,
                    GameType::Rated,
                )
                .is_err()
        );

        seek_service
            .add_seek(
                "player1".to_string(),
                None,
                None,
                game_settings.clone(),
                GameType::Rated,
            )
            .expect("Failed to add seek");

        let sent_messages = mock_client_service.get_broadcasts();
        assert!(sent_messages.len() == 1);
        assert!(matches!(
            &sent_messages[0],
            ServerMessage::SeekList { add: true, seek } if *seek == expected_seek
        ));
        assert!(seek_service.get_seek_ids().len() == 1);
        assert_eq!(seek_service.get_seek(&1).ok(), Some(expected_seek));
    }

    #[test]
    fn test_remove_seek() {
        let mock_client_service = MockClientService::default();
        let mock_game_service = MockGameService::default();
        let seek_service = SeekServiceImpl::new(
            Arc::new(Box::new(mock_client_service.clone())),
            Arc::new(Box::new(mock_game_service.clone())),
        );

        let game_settings = TakGameSettings {
            board_size: 5,
            half_komi: 0,
            reserve_pieces: 21,
            reserve_capstones: 1,
            time_control: TakTimeControl {
                contingent: Duration::from_secs(300),
                increment: Duration::from_secs(5),
                extra: None,
            },
        };

        let expected_seek = Seek {
            id: 1,
            creator: "player1".to_string(),
            opponent: None,
            color: None,
            game_settings: game_settings.clone(),
            game_type: GameType::Rated,
            rematch_from: None,
        };

        seek_service
            .add_seek(
                "player1".to_string(),
                None,
                None,
                game_settings.clone(),
                GameType::Rated,
            )
            .expect("Failed to add seek");

        seek_service
            .remove_seek_of_player(&"player1".to_string())
            .expect("Failed to remove seek");

        let sent_messages = mock_client_service.get_broadcasts();
        assert!(sent_messages.len() == 2);
        assert!(matches!(
            &sent_messages[1],
            ServerMessage::SeekList {
                add: false,
                seek,
            } if *seek == expected_seek
        ));
        assert!(seek_service.get_seek_ids().is_empty());
    }

    #[test]
    fn test_rematch_seek() {
        let mock_client_service = MockClientService::default();
        let mock_game_service = MockGameService::default();
        let seek_service = SeekServiceImpl::new(
            Arc::new(Box::new(mock_client_service.clone())),
            Arc::new(Box::new(mock_game_service.clone())),
        );

        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();

        mock_client_service
            .associate_player(&p1, &"player1".to_string())
            .unwrap();
        mock_client_service
            .associate_player(&p2, &"player2".to_string())
            .unwrap();

        let game_settings = TakGameSettings {
            board_size: 5,
            half_komi: 0,
            reserve_pieces: 21,
            reserve_capstones: 1,
            time_control: TakTimeControl {
                contingent: Duration::from_secs(300),
                increment: Duration::from_secs(5),
                extra: None,
            },
        };

        let expected_seek = Seek {
            id: 1,
            creator: "player1".to_string(),
            opponent: None,
            color: None,
            game_settings: game_settings.clone(),
            game_type: GameType::Rated,
            rematch_from: Some(1),
        };

        seek_service
            .add_rematch_seek(
                "player1".to_string(),
                "player2".to_string(),
                None,
                game_settings.clone(),
                GameType::Rated,
                1,
            )
            .expect("Failed to add seek");

        let sent_broadcasts = mock_client_service.get_broadcasts();
        assert_eq!(sent_broadcasts.len(), 1);
        assert!(matches!(
            &sent_broadcasts[0],
            ServerMessage::SeekList {
                add: true,
                seek,
            } if *seek == expected_seek
        ));
        assert_eq!(seek_service.get_seek_ids().len(), 1);
        assert_eq!(seek_service.get_seek(&1).ok(), Some(expected_seek.clone()));

        seek_service
            .add_rematch_seek(
                "player2".to_string(),
                "player1".to_string(),
                None,
                game_settings.clone(),
                GameType::Rated,
                1,
            )
            .expect("Failed to add seek");

        let sent_messages = mock_client_service.get_messages();
        assert_eq!(sent_messages.len(), 1);
        assert!(matches!(
            &sent_messages[0],
            (uuid, ServerMessage::AcceptRematch { seek_id: 1 }) if *uuid == p2
        ));
        assert_eq!(seek_service.get_seek_ids().len(), 1);
        assert_eq!(seek_service.get_seek(&1).ok(), Some(expected_seek));
    }
}
