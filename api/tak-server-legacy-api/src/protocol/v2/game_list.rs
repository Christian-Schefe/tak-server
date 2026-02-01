use std::time::Instant;

use tak_core::TakTimeSettings;
use tak_player_connection::ConnectionId;
use tak_server_app::{
    domain::{GameId, PlayerId},
    workflow::gameplay::{GameMetadataView, observe::ObserveGameError},
};

use crate::{
    app::ServiceError,
    protocol::{
        Protocol,
        v2::{ProtocolV2Handler, V2Response},
    },
};

impl ProtocolV2Handler {
    pub async fn send_game_list_message(
        &self,
        id: ConnectionId,
        game: &GameMetadataView,
        add: bool,
    ) {
        self.send_game_string_message(
            id,
            game,
            if add {
                "GameList Add"
            } else {
                "GameList Remove"
            },
        )
        .await;
    }

    pub async fn handle_game_list_message(&self, id: ConnectionId) -> V2Response {
        for game in self.app.game_list_ongoing_use_case.list_games() {
            self.send_game_string_message(id, &game.metadata, "GameList Add")
                .await;
        }
        V2Response::OK
    }

    pub async fn handle_observe_message(
        &self,
        id: ConnectionId,
        parts: &[&str],
        observe: bool,
    ) -> V2Response {
        if parts.len() != 2 {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Observe/Unobserve message format".to_string(),
            ));
        }
        let Ok(game_id) = parts[1].parse::<i64>() else {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Game ID in Observe message".to_string(),
            ));
        };
        let game_id = GameId::new(game_id);
        if observe {
            if let Err(e) = self.app.game_observe_use_case.observe_game(game_id, id.0) {
                return match e {
                    ObserveGameError::GameNotFound => V2Response::ErrorNOK(ServiceError::NotFound(
                        "Game ID not found".to_string(),
                    )),
                };
            }
            let Some(game) = self.app.game_get_ongoing_use_case.get_game(game_id) else {
                return V2Response::ErrorNOK(ServiceError::NotFound(
                    "Game ID not found".to_string(),
                ));
            };
            self.send_game_string_message(id, &game.metadata, "Observe")
                .await;
            for action in game.game.action_history() {
                self.send_game_action_message(id, game.metadata.id, action);
            }
            let now = Instant::now();
            let time_info = game.game.get_time_info(now);
            self.send_time_update_message(
                id,
                game_id,
                time_info.white_remaining,
                time_info.black_remaining,
            );
        } else {
            self.app.game_observe_use_case.unobserve_game(game_id, id.0);
        }
        V2Response::OK
    }

    pub async fn send_game_string_message(
        &self,
        id: ConnectionId,
        game: &GameMetadataView,
        operation: &str,
    ) {
        let TakTimeSettings::Realtime(time_control) = &game.settings.time_settings else {
            return;
        };
        let settings = &game.settings;
        let white_account = self
            .app
            .get_account_workflow
            .get_account(game.white_id)
            .await
            .ok();
        let black_account = self
            .app
            .get_account_workflow
            .get_account(game.black_id)
            .await
            .ok();
        let white_username = white_account
            .as_ref()
            .map(|a| a.get_username())
            .unwrap_or("Unknown");
        let black_username = black_account
            .as_ref()
            .map(|a| a.get_username())
            .unwrap_or("Unknown");
        let message = format!(
            "{} {} {} {} {} {} {} {} {} {} {} {} {} {}",
            operation,
            game.id,
            white_username,
            black_username,
            settings.base.board_size,
            time_control.contingent.as_secs(),
            time_control.increment.as_secs(),
            settings.base.half_komi,
            settings.base.reserve.pieces,
            settings.base.reserve.capstones,
            if game.is_rated { "0" } else { "1" }, // protocol has "is_unrated" flag, so invert
            "0",
            time_control
                .extra
                .as_ref()
                .map_or("0".to_string(), |(trigger_move, _)| trigger_move
                    .to_string()),
            time_control
                .extra
                .as_ref()
                .map_or("0".to_string(), |(_, extra_time)| extra_time
                    .as_secs()
                    .to_string()),
        );
        self.send_to(id, message);
    }

    pub async fn send_game_start_message(
        &self,
        id: ConnectionId,
        player_id: PlayerId,
        game: &GameMetadataView,
    ) {
        let TakTimeSettings::Realtime(time_control) = &game.settings.time_settings else {
            return;
        };
        let white_account_id = self
            .app
            .player_resolver_service
            .resolve_account_id_by_player_id(game.white_id)
            .await
            .ok();
        let black_account_id = self
            .app
            .player_resolver_service
            .resolve_account_id_by_player_id(game.black_id)
            .await
            .ok();
        let white_account = if let Some(aid) = &white_account_id {
            self.auth.get_account(aid).await
        } else {
            None
        };
        let black_account = if let Some(aid) = &black_account_id {
            self.auth.get_account(aid).await
        } else {
            None
        };
        let is_white_bot = white_account.as_ref().is_some_and(|a| a.is_bot());
        let is_black_bot = black_account.as_ref().is_some_and(|a| a.is_bot());

        let white_username = white_account
            .as_ref()
            .map(|a| a.get_username())
            .unwrap_or("Unknown");
        let black_username = black_account
            .as_ref()
            .map(|a| a.get_username())
            .unwrap_or("Unknown");

        let is_bot_game = is_white_bot || is_black_bot;
        let settings = &game.settings;
        let protocol = self.transport.get_protocol(id);
        let message = if protocol == Protocol::V0 {
            format!(
                "Game Start {} {} {} vs {} {} {} {} {} {}",
                game.id,
                settings.base.board_size,
                white_username,
                black_username,
                if player_id == game.white_id {
                    "white"
                } else {
                    "black"
                },
                time_control.contingent.as_secs(),
                settings.base.half_komi,
                settings.base.reserve.pieces,
                settings.base.reserve.capstones,
            )
        } else {
            format!(
                "Game Start {} {} vs {} {} {} {} {} {} {} {} {} {} {} {} {}",
                game.id,
                white_username,
                black_username,
                if player_id == game.white_id {
                    "white"
                } else {
                    "black"
                },
                settings.base.board_size,
                time_control.contingent.as_secs(),
                time_control.increment.as_secs(),
                settings.base.half_komi,
                settings.base.reserve.pieces,
                settings.base.reserve.capstones,
                if game.is_rated { "0" } else { "1" }, // protocol has "is_unrated" flag, so invert
                "0",
                time_control
                    .extra
                    .as_ref()
                    .map_or("0".to_string(), |(trigger_move, _)| trigger_move
                        .to_string()),
                time_control
                    .extra
                    .as_ref()
                    .map_or("0".to_string(), |(_, extra_time)| extra_time
                        .as_secs()
                        .to_string()),
                if is_bot_game { "1" } else { "0" }
            )
        };
        self.send_to(id, message);
    }
}
