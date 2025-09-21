use crate::{
    ServiceError, ServiceResult,
    client::ClientId,
    game::GameId,
    player::PlayerUsername,
    protocol::{
        Protocol, ServerGameMessage,
        v2::{ProtocolV2Handler, ProtocolV2Result},
    },
    tak::{TakAction, TakDir, TakGameState, TakPos, TakVariant},
};

impl ProtocolV2Handler {
    pub fn handle_server_game_message(
        &self,
        id: &ClientId,
        game_id: &GameId,
        msg: &ServerGameMessage,
    ) {
        match msg {
            ServerGameMessage::Action(action) => {
                self.send_game_action_message(id, game_id, action);
            }
            ServerGameMessage::GameOver(game_state) => {
                self.send_game_over_message(id, game_id, game_state);
            }
            ServerGameMessage::DrawOffer { offer } => {
                let message = format!(
                    "Game#{} {}",
                    game_id,
                    if *offer { "OfferDraw" } else { "RemoveDraw" }
                );
                self.send_to(id, message);
            }
            ServerGameMessage::UndoRequest { request } => {
                let message = format!(
                    "Game#{} {}",
                    game_id,
                    if *request {
                        "RequestUndo"
                    } else {
                        "RemoveUndo"
                    }
                );
                self.send_to(id, message);
            }
            ServerGameMessage::Undo => {
                let message = format!("Game#{} Undo", game_id);
                self.send_to(id, message);
            }
            ServerGameMessage::TimeUpdate { remaining } => {
                let protocol = self.client_service.get_protocol(id);

                let message = if protocol == Protocol::V0 {
                    format!(
                        "Game#{} Time {} {}",
                        game_id,
                        remaining.0.as_secs(),
                        remaining.1.as_secs()
                    )
                } else {
                    format!(
                        "Game#{} Timems {} {}",
                        game_id,
                        remaining.0.as_millis(),
                        remaining.1.as_millis()
                    )
                };
                self.send_to(id, message);
            }
        }
    }

    pub fn send_game_over_message(
        &self,
        id: &ClientId,
        game_id: &GameId,
        game_state: &TakGameState,
    ) {
        if *game_state == TakGameState::Ongoing {
            return;
        }
        let message = format!("Game#{} Over {}", game_id, game_state.to_string());
        self.send_to(id, message);
    }

    pub fn handle_game_message(
        &self,
        username: &PlayerUsername,
        parts: &[&str],
    ) -> ProtocolV2Result {
        if parts.len() < 2 {
            return ServiceError::bad_request("Invalid Game message format");
        }
        let Some(Ok(game_id)) = parts[0].split("#").nth(1).map(|s| s.parse::<GameId>()) else {
            return ServiceError::bad_request("Invalid Game ID in Game message");
        };

        match parts[1] {
            "P" => self.handle_game_place_message(&username, game_id, &parts[2..])?,
            "M" => self.handle_game_move_message(&username, game_id, &parts[2..])?,
            "Resign" => self.game_service.resign_game(&username, &game_id)?,
            "OfferDraw" => self.game_service.offer_draw(&username, &game_id, true)?,
            "RemoveDraw" => self.game_service.offer_draw(&username, &game_id, false)?,
            "RequestUndo" => self.game_service.request_undo(&username, &game_id, true)?,
            "RemoveUndo" => self.game_service.request_undo(&username, &game_id, false)?,
            _ => return ServiceError::not_found("Unknown Game action"),
        };

        Ok(None)
    }

    pub fn handle_game_place_message(
        &self,
        username: &PlayerUsername,
        game_id: GameId,
        parts: &[&str],
    ) -> ServiceResult<()> {
        if parts.len() != 1 && parts.len() != 2 {
            return ServiceError::bad_request("Invalid Game Place message format");
        }
        let square = parts[0].chars().collect::<Vec<_>>();
        if square.len() != 2 {
            return ServiceError::bad_request("Invalid square format");
        }
        let x = (square[0] as u8).wrapping_sub(b'A') as u32;
        let y = (square[1] as u8).wrapping_sub(b'1') as u32;
        let variant = if parts.len() != 2 {
            TakVariant::Flat
        } else {
            match parts[1] {
                "W" => TakVariant::Standing,
                "C" => TakVariant::Capstone,
                _ => return ServiceError::bad_request("Invalid piece variant"),
            }
        };

        let action = TakAction::Place {
            pos: TakPos::new(x as i32, y as i32),
            variant,
        };

        self.game_service
            .try_do_action(username, &game_id, action)?;

        Ok(())
    }

    pub fn handle_game_move_message(
        &self,
        username: &PlayerUsername,
        game_id: GameId,
        parts: &[&str],
    ) -> ServiceResult<()> {
        if parts.len() < 3 {
            return ServiceError::bad_request("Invalid Game Move message format");
        }
        let from_square = parts[0].chars().collect::<Vec<_>>();
        if from_square.len() != 2 {
            return ServiceError::bad_request("Invalid square format");
        }
        let from_x = (from_square[0] as u8).wrapping_sub(b'A') as u32;
        let from_y = (from_square[1] as u8).wrapping_sub(b'1') as u32;
        let to_square = parts[1].chars().collect::<Vec<_>>();
        if to_square.len() != 2 {
            return ServiceError::bad_request("Invalid square format");
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
            return ServiceError::bad_request("Invalid move direction");
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
        self.game_service
            .try_do_action(username, &game_id, action)?;

        Ok(())
    }

    pub fn send_game_action_message(&self, id: &ClientId, game_id: &GameId, action: &TakAction) {
        let message = match action {
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
        self.send_to(&id, message);
    }
}
