use std::time::Duration;

use crate::{
    app::ServiceError,
    protocol::{
        Protocol,
        v2::{ProtocolV2Handler, V2Response},
    },
};
use tak_core::{
    TakAction, TakActionRecord, TakDir, TakGameState, TakPos, TakVariant, ptn::game_state_to_string,
};
use tak_server_app::domain::{
    GameId, ListenerId, PlayerId,
    game::{DoActionError, OfferDrawError, RequestUndoError, ResignError},
};

impl ProtocolV2Handler {
    pub fn send_draw_offer_message(&self, id: ListenerId, game_id: GameId, offer: bool) {
        let message = format!(
            "Game#{} {}",
            game_id,
            if offer { "OfferDraw" } else { "RemoveDraw" }
        );
        self.send_to(id, message);
    }

    pub fn send_undo_request_message(&self, id: ListenerId, game_id: GameId, offer: bool) {
        let message = format!(
            "Game#{} {}",
            game_id,
            if offer { "RequestUndo" } else { "RemoveUndo" }
        );
        self.send_to(id, message);
    }

    pub fn send_undo_message(&self, id: ListenerId, game_id: GameId) {
        let message = format!("Game#{} Undo", game_id);
        self.send_to(id, message);
    }

    pub fn send_time_update_message(
        &self,
        id: ListenerId,
        game_id: GameId,
        remaining_white: Duration,
        remaining_black: Duration,
    ) {
        let protocol = self.transport.get_protocol(id);

        let message = if protocol == Protocol::V0 {
            format!(
                "Game#{} Time {} {}",
                game_id,
                remaining_white.as_secs(),
                remaining_black.as_secs()
            )
        } else {
            format!(
                "Game#{} Timems {} {}",
                game_id,
                remaining_white.as_millis(),
                remaining_black.as_millis()
            )
        };
        self.send_to(id, message);
    }

    pub fn send_game_over_message(
        &self,
        id: ListenerId,
        game_id: GameId,
        game_state: &TakGameState,
    ) {
        if *game_state == TakGameState::Ongoing {
            return;
        }
        let message = format!("Game#{} Over {}", game_id, game_state_to_string(game_state));
        self.send_to(id, message);
    }

    pub async fn handle_game_message(&self, player_id: PlayerId, parts: &[&str]) -> V2Response {
        if parts.len() < 2 {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Game message format".to_string(),
            ));
        }
        let Some(Ok(game_id)) = parts[0]
            .split("#")
            .nth(1)
            .map(|s| s.parse::<u32>().map(GameId::new))
        else {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Game ID in Game message".to_string(),
            ));
        };

        match parts[1] {
            "P" => {
                if let Err(e) = self
                    .handle_game_place_message(player_id, game_id, &parts[2..])
                    .await
                {
                    let err_str = format!("Error: {}", e);
                    return V2Response::ErrorMessage(e, err_str);
                }
            }
            "M" => {
                if let Err(e) = self
                    .handle_game_move_message(player_id, game_id, &parts[2..])
                    .await
                {
                    let err_str = format!("Error: {}", e);
                    return V2Response::ErrorMessage(e, err_str);
                }
            }
            "Resign" => match self
                .app
                .game_do_action_use_case
                .resign(game_id, player_id)
                .await
            {
                Ok(_) => {}
                Err(ResignError::GameNotFound) => {
                    let err_str = format!("Error: Game not found");
                    return V2Response::ErrorMessage(
                        ServiceError::NotFound("Game not found".to_string()),
                        err_str,
                    );
                }
                Err(ResignError::InvalidResign) => {
                    let err_str = format!("Error: Invalid resign");
                    return V2Response::ErrorMessage(
                        ServiceError::BadRequest("Invalid resign".to_string()),
                        err_str,
                    );
                }
                Err(ResignError::NotPlayersTurn) => {
                    let err_str = format!("Error: Not player's turn");
                    return V2Response::ErrorMessage(
                        ServiceError::BadRequest("Not player's turn".to_string()),
                        err_str,
                    );
                }
            },
            "OfferDraw" => match self
                .app
                .game_do_action_use_case
                .offer_draw(game_id, player_id)
                .await
            {
                Ok(_) => {}
                Err(OfferDrawError::GameNotFound) => {
                    let err_str = format!("Error: Game not found");
                    return V2Response::ErrorMessage(
                        ServiceError::NotFound("Game not found".to_string()),
                        err_str,
                    );
                }
                Err(OfferDrawError::InvalidOffer) => {
                    let err_str = format!("Error: Invalid draw offer");
                    return V2Response::ErrorMessage(
                        ServiceError::BadRequest("Invalid draw offer".to_string()),
                        err_str,
                    );
                }
                Err(OfferDrawError::NotAPlayerInGame) => {
                    let err_str = format!("Error: Not a player in game");
                    return V2Response::ErrorMessage(
                        ServiceError::BadRequest("Not a player in game".to_string()),
                        err_str,
                    );
                }
            },
            "RemoveDraw" => match self
                .app
                .game_do_action_use_case
                .offer_draw(game_id, player_id)
                .await
            {
                Ok(_) => {}
                Err(OfferDrawError::GameNotFound) => {
                    let err_str = format!("Error: Game not found");
                    return V2Response::ErrorMessage(
                        ServiceError::NotFound("Game not found".to_string()),
                        err_str,
                    );
                }
                Err(OfferDrawError::InvalidOffer) => {
                    let err_str = format!("Error: Invalid draw removal");
                    return V2Response::ErrorMessage(
                        ServiceError::BadRequest("Invalid draw removal".to_string()),
                        err_str,
                    );
                }
                Err(OfferDrawError::NotAPlayerInGame) => {
                    let err_str = format!("Error: Not a player in game");
                    return V2Response::ErrorMessage(
                        ServiceError::BadRequest("Not a player in game".to_string()),
                        err_str,
                    );
                }
            },
            "RequestUndo" => match self
                .app
                .game_do_action_use_case
                .request_undo(game_id, player_id)
                .await
            {
                Ok(_) => {}
                Err(RequestUndoError::GameNotFound) => {
                    let err_str = format!("Error: Game not found");
                    return V2Response::ErrorMessage(
                        ServiceError::NotFound("Game not found".to_string()),
                        err_str,
                    );
                }
                Err(RequestUndoError::InvalidRequest) => {
                    let err_str = format!("Error: Invalid undo request");
                    return V2Response::ErrorMessage(
                        ServiceError::BadRequest("Invalid undo request".to_string()),
                        err_str,
                    );
                }
                Err(RequestUndoError::NotAPlayerInGame) => {
                    let err_str = format!("Error: Not a player in game");
                    return V2Response::ErrorMessage(
                        ServiceError::BadRequest("Not a player in game".to_string()),
                        err_str,
                    );
                }
            },
            "RemoveUndo" => match self
                .app
                .game_do_action_use_case
                .retract_undo_request(game_id, player_id)
                .await
            {
                Ok(_) => {}
                Err(RequestUndoError::GameNotFound) => {
                    let err_str = format!("Error: Game not found");
                    return V2Response::ErrorMessage(
                        ServiceError::NotFound("Game not found".to_string()),
                        err_str,
                    );
                }
                Err(RequestUndoError::InvalidRequest) => {
                    let err_str = format!("Error: Invalid undo retraction");
                    return V2Response::ErrorMessage(
                        ServiceError::BadRequest("Invalid undo retraction".to_string()),
                        err_str,
                    );
                }
                Err(RequestUndoError::NotAPlayerInGame) => {
                    let err_str = format!("Error: Not a player in game");
                    return V2Response::ErrorMessage(
                        ServiceError::BadRequest("Not a player in game".to_string()),
                        err_str,
                    );
                }
            },
            _ => {
                return V2Response::ErrorNOK(ServiceError::BadRequest(
                    "Unknown Game command".to_string(),
                ));
            }
        };

        V2Response::OK
    }

    pub async fn handle_game_place_message(
        &self,
        player_id: PlayerId,
        game_id: GameId,
        parts: &[&str],
    ) -> Result<(), ServiceError> {
        if parts.len() != 1 && parts.len() != 2 {
            return Err(ServiceError::BadRequest(
                "Invalid Game Place message format".to_string(),
            ));
        }
        let square = parts[0].chars().collect::<Vec<_>>();
        if square.len() != 2 {
            return Err(ServiceError::BadRequest(
                "Invalid square format".to_string(),
            ));
        }
        let x = (square[0] as u8).wrapping_sub(b'A') as u32;
        let y = (square[1] as u8).wrapping_sub(b'1') as u32;
        let variant = if parts.len() != 2 {
            TakVariant::Flat
        } else {
            match parts[1] {
                "W" => TakVariant::Standing,
                "C" => TakVariant::Capstone,
                _ => {
                    return Err(ServiceError::BadRequest(
                        "Invalid piece variant".to_string(),
                    ));
                }
            }
        };

        let action = TakAction::Place {
            pos: TakPos::new(x as i32, y as i32),
            variant,
        };

        match self
            .app
            .game_do_action_use_case
            .do_action(game_id, player_id, action)
            .await
        {
            Ok(_) => {}
            Err(DoActionError::GameNotFound) => {
                return Err(ServiceError::NotFound("Game not found".to_string()));
            }
            Err(DoActionError::InvalidAction) => {
                return Err(ServiceError::BadRequest("Invalid action".to_string()));
            }
            Err(DoActionError::NotPlayersTurn) => {
                return Err(ServiceError::BadRequest("Not player's turn".to_string()));
            }
        }

        Ok(())
    }

    pub async fn handle_game_move_message(
        &self,
        player_id: PlayerId,
        game_id: GameId,
        parts: &[&str],
    ) -> Result<(), ServiceError> {
        if parts.len() < 3 {
            return Err(ServiceError::BadRequest(
                "Invalid Game Move message format".to_string(),
            ));
        }
        let from_square = parts[0].chars().collect::<Vec<_>>();
        if from_square.len() != 2 {
            return Err(ServiceError::BadRequest(
                "Invalid square format".to_string(),
            ));
        }
        let from_x = (from_square[0] as u8).wrapping_sub(b'A') as u32;
        let from_y = (from_square[1] as u8).wrapping_sub(b'1') as u32;
        let to_square = parts[1].chars().collect::<Vec<_>>();
        if to_square.len() != 2 {
            return Err(ServiceError::BadRequest(
                "Invalid square format".to_string(),
            ));
        }
        let to_x = (to_square[0] as u8).wrapping_sub(b'A') as u32;
        let to_y = (to_square[1] as u8).wrapping_sub(b'1') as u32;
        let dir = if from_x == to_x {
            if to_y > from_y {
                TakDir::Up
            } else {
                TakDir::Down
            }
        } else if from_y == to_y {
            if to_x > from_x {
                TakDir::Right
            } else {
                TakDir::Left
            }
        } else {
            return Err(ServiceError::BadRequest(
                "Invalid move direction".to_string(),
            ));
        };
        let drops = parts[2..]
            .iter()
            .map(|s| {
                s.parse::<u32>()
                    .map_err(|_| ServiceError::BadRequest("Invalid drop count".into()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let action = TakAction::Move {
            pos: TakPos::new(from_x as i32, from_y as i32),
            dir,
            drops,
        };
        match self
            .app
            .game_do_action_use_case
            .do_action(game_id, player_id, action)
            .await
        {
            Ok(_) => {}
            Err(DoActionError::GameNotFound) => {
                return Err(ServiceError::NotFound("Game not found".to_string()));
            }
            Err(DoActionError::InvalidAction) => {
                return Err(ServiceError::BadRequest("Invalid action".to_string()));
            }
            Err(DoActionError::NotPlayersTurn) => {
                return Err(ServiceError::BadRequest("Not player's turn".to_string()));
            }
        };

        Ok(())
    }

    pub fn send_game_action_message(
        &self,
        id: ListenerId,
        game_id: GameId,
        action: &TakActionRecord,
    ) {
        let message = match &action.action {
            TakAction::Place { pos, variant } => format!(
                "Game#{} P {}{} {}",
                game_id,
                (b'A' + pos.x as u8) as char,
                (b'1' + pos.y as u8) as char,
                match variant {
                    TakVariant::Flat => "",
                    TakVariant::Standing => "W",
                    TakVariant::Capstone => "C",
                }
            ),
            TakAction::Move { pos, dir, drops } => {
                let end_pos = pos.offset(dir, drops.len() as i32);
                let drops_str = drops
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!(
                    "Game#{} M {}{} {}{} {}",
                    game_id,
                    (b'A' + pos.x as u8) as char,
                    (b'1' + pos.y as u8) as char,
                    (b'A' + end_pos.x as u8) as char,
                    (b'1' + end_pos.y as u8) as char,
                    drops_str
                )
            }
        };
        self.send_to(id, message);
    }
}
