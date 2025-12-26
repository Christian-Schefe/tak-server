use std::time::Duration;

use crate::{
    acl::get_player_id_by_username,
    app::ServiceError,
    protocol::v2::{ProtocolV2Handler, V2Response},
};

use tak_core::{TakGameSettings, TakPlayer, TakReserve, TakTimeControl};
use tak_server_app::domain::{GameId, GameType, ListenerId, PlayerId};
use tak_server_app::workflow::matchmaking::create::CreateSeekError;

impl ProtocolV2Handler {
    pub async fn handle_seek_message(&self, player_id: PlayerId, parts: &[&str]) -> V2Response {
        match self.handle_add_seek_message(player_id, parts).await {
            Ok(()) => V2Response::OK,
            Err(e) => V2Response::ErrorNOK(e),
        }
    }

    pub async fn handle_seek_list_message(&self, id: ListenerId) -> V2Response {
        for seek in self.app.seek_list_use_case.list_seeks() {
            self.handle_server_seek_list_message(id, &seek, true).await;
        }
        V2Response::OK
    }

    // TODO: Support V0 seek messages
    async fn handle_add_seek_message(
        &self,
        player_id: PlayerId,
        parts: &[&str],
    ) -> Result<(), ServiceError> {
        if parts.len() != 13 && parts.len() != 12 && parts.len() != 11 && parts.len() != 10 {
            return Err(ServiceError::BadRequest(
                "Invalid Seek message format".to_string(),
            ));
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
            (true, false) => GameType::Unrated,
            (false, false) => GameType::Rated,
            (_, true) => GameType::Tournament,
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
            reserve: TakReserve::new(reserve_pieces, reserve_capstones),
            time_control: TakTimeControl {
                contingent: Duration::from_secs(time_contingent_seconds as u64),
                increment: Duration::from_secs(time_increment_seconds as u64),
                extra: time_extra,
            },
        };

        if !game_settings.is_valid() {
            self.app.seek_cancel_use_case.cancel_seek(player_id);
        } else {
            let opponent_id = match opponent {
                Some(ref name) => match get_player_id_by_username(name) {
                    Some(id) => Some(id),
                    None => {
                        return Err(ServiceError::BadRequest(format!("No such user: {}", name)));
                    }
                },
                None => None,
            };
            if let Err(e) = self.app.seek_create_use_case.create_seek(
                player_id,
                opponent_id,
                color,
                game_settings,
                game_type,
            ) {
                match e {
                    CreateSeekError::InvalidGameSettings => {
                        return Err(ServiceError::BadRequest(
                            "Invalid game settings for seek".into(),
                        ));
                    }
                    CreateSeekError::InvalidOpponent => {
                        return Err(ServiceError::BadRequest("Invalid opponent for seek".into()));
                    }
                }
            }
        }
        Ok(())
    }
    /*
    } else if let Some(from_game) = rematch {
        let Some(opponent) = opponent else {
            return Err(ServiceError::BadRequest(
                "Rematch seek must specify opponent".into(),
            ));
        };
        if let Some(new_seek_id) = self
            .app
            .match_rematch_use_case
            .request_rematch(
                username.to_string(),
                opponent,
                color,
                game_settings,
                game_type,
                from_game,
            )
            .await?
        {
            return Ok(V2Response::Message(format!(
                "Rematch seek created with ID: {}",
                new_seek_id
            )));
        } */
    pub async fn handle_rematch_message(&self, player_id: &PlayerId, parts: &[&str]) -> V2Response {
        if parts.len() < 2 {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Rematch message format".to_string(),
            ));
        }
        let Ok(game_id) = parts[1].parse::<GameId>() else {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Game ID in Rematch message".to_string(),
            ));
        };
        self.handle_add_seek_message(username, &parts[1..], Some(game_id))
            .await
    }

    pub async fn handle_accept_message(
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
        self.app
            .seek_service
            .accept_seek(username, &seek_id)
            .await?;
        Ok(V2Response::OK)
    }

    pub async fn handle_server_seek_list_message(&self, id: ListenerId, seek: &Seek, add: bool) {
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
            seek.game_settings.reserve.pieces,
            seek.game_settings.reserve.capstones,
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
            self.app
                .player_service
                .get_player(&seek.creator)
                .await
                .map_or("0", |p| if p.flags.is_bot { "1" } else { "0" })
        );
        self.send_to(id, message);
    }
}
