use std::time::Duration;

use crate::{
    ServiceError,
    client::ClientId,
    game::GameId,
    player::PlayerUsername,
    protocol::v2::{ProtocolV2Handler, ProtocolV2Result},
    seek::{GameType, Seek},
};

use tak_core::{TakGameSettings, TakPlayer, TakTimeControl};

impl ProtocolV2Handler {
    pub fn handle_seek_message(
        &self,
        username: &PlayerUsername,
        parts: &[&str],
    ) -> ProtocolV2Result {
        self.handle_add_seek_message(username, parts, None)
    }

    pub fn handle_seek_list_message(&self, id: &ClientId) -> ProtocolV2Result {
        for seek in self.seek_service.get_seeks() {
            self.handle_server_seek_list_message(id, &seek, true);
        }
        Ok(None)
    }

    // TODO: Support V0 seek messages
    pub fn handle_add_seek_message(
        &self,
        username: &PlayerUsername,
        parts: &[&str],
        rematch: Option<GameId>,
    ) -> ProtocolV2Result {
        if parts.len() != 13 && parts.len() != 12 && parts.len() != 11 && parts.len() != 10 {
            return ServiceError::bad_request("Invalid Seek message format");
        }
        let board_size = parts[1]
            .parse::<u32>()
            .map_err(|_| ServiceError::BadRequest("Invalid board size".into()))?;
        let time_contingent_seconds = parts[2]
            .parse::<u32>()
            .map_err(|_| ServiceError::BadRequest("Invalid time contingent".into()))?;
        let time_increment_seconds = parts[3]
            .parse::<u32>()
            .map_err(|_| ServiceError::BadRequest("Invalid time increment".into()))?;
        let color = match parts[4] {
            "W" => Some(TakPlayer::White),
            "B" => Some(TakPlayer::Black),
            "A" => None,
            _ => return Err(ServiceError::BadRequest("Invalid color".into())),
        };
        let half_komi = parts[5]
            .parse::<u32>()
            .map_err(|_| ServiceError::BadRequest("Invalid half komi".into()))?;
        let reserve_pieces = parts[6]
            .parse::<u32>()
            .map_err(|_| ServiceError::BadRequest("Invalid reserve pieces".into()))?;
        let reserve_capstones = parts[7]
            .parse::<u32>()
            .map_err(|_| ServiceError::BadRequest("Invalid reserve capstones".into()))?;
        let is_unrated = match parts[8] {
            "1" => true,
            "0" => false,
            _ => {
                return Err(ServiceError::BadRequest(
                    "Invalid rated/unrated flag".into(),
                ));
            }
        };
        let is_tournament = match parts[9] {
            "1" => true,
            "0" => false,
            _ => return Err(ServiceError::BadRequest("Invalid tournament flag".into())),
        };
        let game_type = match (is_unrated, is_tournament) {
            (true, false) => crate::seek::GameType::Unrated,
            (false, false) => crate::seek::GameType::Rated,
            (_, true) => crate::seek::GameType::Tournament,
        };
        let time_extra_trigger_move = if parts.len() >= 12 {
            parts[10]
                .parse::<u32>()
                .map_err(|_| ServiceError::BadRequest("Invalid time extra trigger move".into()))?
        } else {
            0
        };
        let time_extra_trigger_seconds = if parts.len() >= 12 {
            parts[11].parse::<u32>().map_err(|_| {
                ServiceError::BadRequest("Invalid time extra trigger seconds".into())
            })?
        } else {
            0
        };
        let opponent = if parts.len() == 13 {
            Some(parts[12].to_string())
        } else if parts.len() == 11 {
            Some(parts[9].to_string())
        } else {
            None
        };

        let time_extra = if time_extra_trigger_move > 0 && time_extra_trigger_seconds > 0 {
            Some((
                time_extra_trigger_move,
                Duration::from_secs(time_extra_trigger_seconds as u64),
            ))
        } else {
            None
        };

        let game_settings = TakGameSettings {
            board_size,
            half_komi,
            reserve_pieces,
            reserve_capstones,
            time_control: TakTimeControl {
                contingent: Duration::from_secs(time_contingent_seconds as u64),
                increment: Duration::from_secs(time_increment_seconds as u64),
                extra: time_extra,
            },
        };

        if !game_settings.is_valid() {
            self.seek_service.remove_seek_of_player(&username)?;
        } else if let Some(from_game) = rematch {
            let Some(opponent) = opponent else {
                return Err(ServiceError::BadRequest(
                    "Rematch seek must specify opponent".into(),
                ));
            };
            self.seek_service.add_rematch_seek(
                username.to_string(),
                opponent,
                color,
                game_settings,
                game_type,
                from_game,
            )?;
        } else {
            self.seek_service.add_seek(
                username.to_string(),
                opponent,
                color,
                game_settings,
                game_type,
            )?;
        }

        Ok(None)
    }

    pub fn handle_rematch_message(
        &self,
        username: &PlayerUsername,
        parts: &[&str],
    ) -> ProtocolV2Result {
        if parts.len() < 2 {
            return ServiceError::bad_request("Invalid Rematch message format");
        }
        let Ok(game_id) = parts[1].parse::<GameId>() else {
            return ServiceError::bad_request("Invalid Game ID in Rematch message");
        };
        self.handle_add_seek_message(username, &parts[1..], Some(game_id))?;
        Ok(None)
    }

    pub fn handle_accept_message(
        &self,
        username: &PlayerUsername,
        parts: &[&str],
    ) -> ProtocolV2Result {
        if parts.len() != 2 {
            return ServiceError::bad_request("Invalid Accept message format");
        }
        let Ok(seek_id) = parts[1].parse::<u32>() else {
            return ServiceError::bad_request("Invalid Seek ID in Accept message");
        };
        self.seek_service.accept_seek(username, &seek_id)?;
        Ok(None)
    }

    pub fn handle_server_seek_list_message(&self, id: &ClientId, seek: &Seek, add: bool) {
        let message = format!(
            "Seek {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
            if add { "new" } else { "remove" },
            seek.id,
            seek.creator,
            seek.game_settings.board_size,
            seek.game_settings.time_control.contingent.as_secs(),
            seek.game_settings.time_control.increment.as_secs(),
            seek.color.as_ref().map_or("A", |c| match c {
                TakPlayer::White => "W",
                TakPlayer::Black => "B",
            }),
            seek.game_settings.half_komi,
            seek.game_settings.reserve_pieces,
            seek.game_settings.reserve_capstones,
            match seek.game_type {
                GameType::Unrated => "1",
                GameType::Rated => "0",
                GameType::Tournament => "0",
            },
            match seek.game_type {
                GameType::Unrated => "0",
                GameType::Rated => "0",
                GameType::Tournament => "1",
            },
            seek.game_settings
                .time_control
                .extra
                .as_ref()
                .map_or("0".to_string(), |(trigger_move, _)| trigger_move
                    .to_string()),
            seek.game_settings
                .time_control
                .extra
                .as_ref()
                .map_or("0".to_string(), |(_, extra_time)| extra_time
                    .as_secs()
                    .to_string()),
            seek.opponent.as_deref().unwrap_or(""),
            self.player_service
                .fetch_player(&seek.creator)
                .map_or("0", |p| if p.is_bot { "1" } else { "0" })
        );
        self.send_to(id, message);
    }
}
