use std::time::Instant;

use crate::{
    ServiceError,
    client::ClientId,
    game::{Game, GameId},
    protocol::{
        Protocol, ServerGameMessage, ServerMessage,
        v2::{ProtocolV2Handler, ProtocolV2Result},
    },
    seek::GameType,
};

impl ProtocolV2Handler {
    pub fn handle_server_game_list_message(&self, id: &ClientId, msg: &ServerMessage) {
        match msg {
            ServerMessage::GameList { add, game } => {
                self.send_game_string_message(
                    id,
                    game,
                    if *add {
                        "GameList Add"
                    } else {
                        "GameList Remove"
                    },
                );
            }
            ServerMessage::GameStart { game } => {
                self.send_game_start_message(id, game);
            }
            ServerMessage::ObserveGame { game } => {
                self.send_game_string_message(id, game, "Observe");
                for action in game.game.action_history.iter() {
                    self.send_game_action_message(id, &game.id, action);
                }
                let now = Instant::now();
                let remaining = game.game.get_time_remaining_both(now);
                self.handle_server_game_message(
                    id,
                    &game.id,
                    &ServerGameMessage::TimeUpdate { remaining },
                );
            }
            _ => {
                eprintln!("Unhandled server game list message: {:?}", msg);
            }
        }
    }

    pub fn handle_game_list_message(&self, id: &ClientId) -> ProtocolV2Result {
        for game in self.game_service.get_games() {
            self.send_game_string_message(id, &game, "GameList Add");
        }
        Ok(None)
    }

    pub fn handle_observe_message(
        &self,
        id: &ClientId,
        parts: &[&str],
        observe: bool,
    ) -> ProtocolV2Result {
        if parts.len() != 2 {
            return ServiceError::bad_request("Invalid Observe/Unobserve message format");
        }
        let Ok(game_id) = parts[1].parse::<GameId>() else {
            return ServiceError::bad_request("Invalid Game ID in Observe message");
        };
        if observe {
            self.game_service.observe_game(id, &game_id)?;
        } else {
            self.game_service.unobserve_game(id, &game_id)?;
        }
        Ok(None)
    }

    pub fn send_game_string_message(&self, id: &ClientId, game: &Game, operation: &str) {
        let settings = &game.game.base.settings;
        let message = format!(
            "{} {} {} {} {} {} {} {} {} {} {} {} {} {}",
            operation,
            game.id,
            game.white,
            game.black,
            settings.board_size,
            settings.time_control.contingent.as_secs(),
            settings.time_control.increment.as_secs(),
            settings.half_komi,
            settings.reserve_pieces,
            settings.reserve_capstones,
            match game.game_type {
                GameType::Unrated => "1",
                GameType::Rated => "0",
                GameType::Tournament => "0",
            },
            match game.game_type {
                GameType::Unrated => "0",
                GameType::Rated => "0",
                GameType::Tournament => "1",
            },
            settings
                .time_control
                .extra
                .as_ref()
                .map_or("0".to_string(), |(_, extra_time)| extra_time
                    .as_secs()
                    .to_string()),
            settings
                .time_control
                .extra
                .as_ref()
                .map_or("0".to_string(), |(trigger_move, _)| trigger_move
                    .to_string()),
        );
        self.send_to(id, message);
    }

    pub fn send_game_start_message(&self, id: &ClientId, game: &Game) {
        let Some(player) = self.client_service.get_associated_player(&id) else {
            println!("Client {} not associated with any player", id);
            return;
        };
        let is_bot_game = self
            .player_service
            .fetch_player(&game.white)
            .map_or(false, |p| p.is_bot)
            || self
                .player_service
                .fetch_player(&game.black)
                .map_or(false, |p| p.is_bot);
        let settings = &game.game.base.settings;
        let protocol = self.client_service.get_protocol(id);
        let message = if protocol == Protocol::V0 {
            format!(
                "Game Start {} {} {} vs {} {} {} {} {} {}",
                game.id,
                settings.board_size,
                game.white,
                game.black,
                if *player == game.white {
                    "white"
                } else {
                    "black"
                },
                settings.time_control.contingent.as_secs(),
                settings.half_komi,
                settings.reserve_pieces,
                settings.reserve_capstones,
            )
        } else {
            format!(
                "Game Start {} {} vs {} {} {} {} {} {} {} {} {} {} {} {} {}",
                game.id,
                game.white,
                game.black,
                if *player == game.white {
                    "white"
                } else {
                    "black"
                },
                settings.board_size,
                settings.time_control.contingent.as_secs(),
                settings.time_control.increment.as_secs(),
                settings.half_komi,
                settings.reserve_pieces,
                settings.reserve_capstones,
                match game.game_type {
                    GameType::Unrated => "1",
                    GameType::Rated => "0",
                    GameType::Tournament => "0",
                },
                match game.game_type {
                    GameType::Unrated => "0",
                    GameType::Rated => "0",
                    GameType::Tournament => "1",
                },
                settings
                    .time_control
                    .extra
                    .as_ref()
                    .map_or("0".to_string(), |(trigger_move, _)| trigger_move
                        .to_string()),
                settings
                    .time_control
                    .extra
                    .as_ref()
                    .map_or("0".to_string(), |(_, extra_time)| extra_time
                        .as_secs()
                        .to_string()),
                if is_bot_game { "1" } else { "0" }
            )
        };
        self.send_to(&id, message);
    }
}
