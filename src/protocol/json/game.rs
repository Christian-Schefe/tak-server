use tak_core::ptn::ptn_to_action;

use crate::{
    ServiceError, ServiceResult,
    game::GameId,
    player::PlayerUsername,
    protocol::json::{ClientResponse, ProtocolJsonHandler},
};

impl ProtocolJsonHandler {
    pub fn handle_game_action(
        &self,
        username: &PlayerUsername,
        game_id: &GameId,
        action: &str,
    ) -> ServiceResult<ClientResponse> {
        let Some(action) = ptn_to_action(&action) else {
            return ServiceError::bad_request("Invalid action PTN");
        };
        self.game_service.try_do_action(username, game_id, action)?;
        Ok(ClientResponse::Ok)
    }

    pub fn handle_undo_request_message(
        &self,
        username: &PlayerUsername,
        game_id: &GameId,
        request: bool,
    ) -> ServiceResult<ClientResponse> {
        self.game_service.request_undo(username, game_id, request)?;
        Ok(ClientResponse::Ok)
    }

    pub fn handle_draw_offer_message(
        &self,
        username: &PlayerUsername,
        game_id: &GameId,
        offer: bool,
    ) -> ServiceResult<ClientResponse> {
        self.game_service.offer_draw(username, game_id, offer)?;
        Ok(ClientResponse::Ok)
    }

    pub fn handle_resign_message(
        &self,
        username: &PlayerUsername,
        game_id: &GameId,
    ) -> ServiceResult<ClientResponse> {
        self.game_service.resign_game(username, game_id)?;
        Ok(ClientResponse::Ok)
    }
}
