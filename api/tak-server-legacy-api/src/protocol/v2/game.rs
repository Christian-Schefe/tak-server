use std::time::Duration;

use crate::{
    app::ServiceError,
    protocol::{
        Protocol,
        v2::{ProtocolV2Handler, V2Response},
    },
};
use tak_core::{
    TakAction, TakDir, TakGameResult, TakPos, TakRequestType, TakVariant,
    ptn::game_result_to_string,
};
use tak_player_connection::ConnectionId;
use tak_server_app::{
    domain::{GameId, PlayerId},
    workflow::gameplay::do_action::{
        ActionResult, AddRequestError, DoActionError, HandleRequestError, PlayerActionError,
    },
};

impl ProtocolV2Handler {
    pub fn send_draw_offer_message(&self, id: ConnectionId, game_id: GameId, offer: bool) {
        let message = format!(
            "Game#{} {}",
            game_id,
            if offer { "OfferDraw" } else { "RemoveDraw" }
        );
        self.send_to(id, message);
    }

    pub fn send_undo_request_message(&self, id: ConnectionId, game_id: GameId, offer: bool) {
        let message = format!(
            "Game#{} {}",
            game_id,
            if offer { "RequestUndo" } else { "RemoveUndo" }
        );
        self.send_to(id, message);
    }

    pub fn send_undo_message(&self, id: ConnectionId, game_id: GameId) {
        let message = format!("Game#{} Undo", game_id);
        self.send_to(id, message);
    }

    pub fn send_time_update_message(
        &self,
        id: ConnectionId,
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
        id: ConnectionId,
        game_id: GameId,
        game_result: &TakGameResult,
    ) {
        let message = format!(
            "Game#{} Over {}",
            game_id,
            game_result_to_string(game_result)
        );
        self.send_to(id, message);
    }

    pub async fn handle_game_message(
        &self,
        id: ConnectionId,
        player_id: PlayerId,
        parts: &[&str],
    ) -> V2Response {
        if parts.len() < 2 {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Game message format".to_string(),
            ));
        }
        let Some(Ok(game_id)) = parts[0]
            .split("#")
            .nth(1)
            .map(|s| s.parse::<i64>().map(GameId::new))
        else {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Game ID in Game message".to_string(),
            ));
        };

        let Some(game) = self.app.game_get_ongoing_use_case.get_game(game_id) else {
            return V2Response::ErrorNOK(ServiceError::NotFound("Game not found".to_string()));
        };
        let opponent_id = if game.metadata.white_id == player_id {
            game.metadata.black_id
        } else if game.metadata.black_id == player_id {
            game.metadata.white_id
        } else {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "You are not a player in this game".to_string(),
            ));
        };

        match parts[1] {
            "P" => {
                self.connection_that_did_last_move
                    .insert((game_id, player_id), id);
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
                    self.connection_that_did_last_move
                        .insert((game_id, player_id), id);
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
                Err(e) => return player_action_error(e),
            },
            "OfferDraw" => {
                return self
                    .add_or_accept_request(game_id, player_id, opponent_id, TakRequestType::Draw)
                    .await;
            }
            "RemoveDraw" => {
                return self
                    .retract_request(game_id, player_id, TakRequestType::Draw)
                    .await;
            }
            "RequestUndo" => {
                return self
                    .add_or_accept_request(game_id, player_id, opponent_id, TakRequestType::Undo)
                    .await;
            }
            "RemoveUndo" => {
                return self
                    .retract_request(game_id, player_id, TakRequestType::Undo)
                    .await;
            }
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
            ActionResult::Success => {}
            ActionResult::NotPossible(PlayerActionError::GameNotFound) => {
                return Err(ServiceError::NotFound("Game not found".to_string()));
            }
            ActionResult::NotPossible(PlayerActionError::NotAPlayerInGame) => {
                return Err(ServiceError::BadRequest("Not a player in game".to_string()));
            }
            ActionResult::ActionError(DoActionError::InvalidAction(reason)) => {
                return Err(ServiceError::BadRequest(format!(
                    "Invalid action: {:?}",
                    reason
                )));
            }
            ActionResult::ActionError(DoActionError::NotPlayersTurn) => {
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
            ActionResult::Success => {}
            ActionResult::NotPossible(PlayerActionError::GameNotFound) => {
                return Err(ServiceError::NotFound("Game not found".to_string()));
            }
            ActionResult::NotPossible(PlayerActionError::NotAPlayerInGame) => {
                return Err(ServiceError::BadRequest("Not a player in game".to_string()));
            }
            ActionResult::ActionError(DoActionError::InvalidAction(reason)) => {
                return Err(ServiceError::BadRequest(format!(
                    "Invalid action: {:?}",
                    reason
                )));
            }
            ActionResult::ActionError(DoActionError::NotPlayersTurn) => {
                return Err(ServiceError::BadRequest("Not player's turn".to_string()));
            }
        };

        Ok(())
    }

    pub fn send_game_action_message(&self, id: ConnectionId, game_id: GameId, action: &TakAction) {
        let message = match &action {
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

    async fn add_or_accept_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        opponent_id: PlayerId,
        request_type: TakRequestType,
    ) -> V2Response {
        let requests = self
            .app
            .game_do_action_use_case
            .get_requests_of_player(game_id, opponent_id);
        if let Some(request_id) = requests.and_then(|reqs| {
            reqs.into_iter()
                .find_map(|r| match (&request_type, &r.request_type) {
                    (TakRequestType::Draw, TakRequestType::Draw) => Some(r.id),
                    (TakRequestType::Undo, TakRequestType::Undo) => Some(r.id),
                    _ => None,
                })
        }) {
            let res = match &request_type {
                TakRequestType::Draw => {
                    self.app
                        .game_do_action_use_case
                        .accept_draw_request(game_id, player_id, request_id)
                        .await
                }
                TakRequestType::Undo => {
                    self.app
                        .game_do_action_use_case
                        .accept_undo_request(game_id, player_id, request_id)
                        .await
                }
                _ => {
                    return V2Response::ErrorNOK(ServiceError::BadRequest(
                        "Invalid request type".to_string(),
                    ));
                }
            };
            match res {
                ActionResult::Success => {}
                ActionResult::NotPossible(e) => return player_action_error(e),
                ActionResult::ActionError(HandleRequestError::RequestNotFound) => {
                    let err_str = format!("Error: Request not found");
                    return V2Response::ErrorMessage(
                        ServiceError::NotFound("Request not found".to_string()),
                        err_str,
                    );
                }
            }
        } else {
            match self
                .app
                .game_do_action_use_case
                .add_request(game_id, player_id, request_type)
                .await
            {
                ActionResult::Success => {}
                ActionResult::NotPossible(e) => return player_action_error(e),
                ActionResult::ActionError(AddRequestError::AlreadyRequested) => {
                    let err_str = format!("Error: Already requested");
                    return V2Response::ErrorMessage(
                        ServiceError::NotFound("Already requested".to_string()),
                        err_str,
                    );
                }
            }
        }
        V2Response::OK
    }

    async fn retract_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        request_type: TakRequestType,
    ) -> V2Response {
        let requests = self
            .app
            .game_do_action_use_case
            .get_requests_of_player(game_id, player_id);
        let Some(request_id) = requests.and_then(|reqs| {
            reqs.into_iter()
                .find_map(|r| match (&request_type, &r.request_type) {
                    (TakRequestType::Draw, TakRequestType::Draw) => Some(r.id),
                    (TakRequestType::Undo, TakRequestType::Undo) => Some(r.id),
                    _ => None,
                })
        }) else {
            let err_str = format!("Error: No request to remove");
            return V2Response::ErrorMessage(
                ServiceError::NotFound("No request to remove".to_string()),
                err_str,
            );
        };
        match self
            .app
            .game_do_action_use_case
            .retract_request(game_id, player_id, request_id)
            .await
        {
            ActionResult::Success => V2Response::OK,
            ActionResult::ActionError(HandleRequestError::RequestNotFound) => {
                let err_str = format!("Error: Request not found");
                return V2Response::ErrorMessage(
                    ServiceError::NotFound("Request not found".to_string()),
                    err_str,
                );
            }
            ActionResult::NotPossible(e) => return player_action_error(e),
        }
    }
}

fn player_action_error(err: PlayerActionError) -> V2Response {
    match err {
        PlayerActionError::GameNotFound => {
            let err_str = format!("Error: Game not found");
            return V2Response::ErrorMessage(
                ServiceError::NotFound("Game not found".to_string()),
                err_str,
            );
        }
        PlayerActionError::NotAPlayerInGame => {
            let err_str = format!("Error: Not a player in game");
            return V2Response::ErrorMessage(
                ServiceError::BadRequest("Not a player in game".to_string()),
                err_str,
            );
        }
    }
}
