use tak_core::ptn::action_from_ptn;
use tak_server_domain::{ServiceError, ServiceResult, game::GameId, player::PlayerUsername};

use crate::protocol::json::{ClientResponse, ProtocolJsonHandler};

impl ProtocolJsonHandler {
    pub async fn handle_game_action(
        &self,
        username: &PlayerUsername,
        game_id: &GameId,
        action: &str,
    ) -> ServiceResult<ClientResponse> {
        let Some(action) = action_from_ptn(&action) else {
            return ServiceError::bad_request("Invalid action PTN");
        };
        self.game_service
            .try_do_action(username, game_id, action)
            .await?;
        Ok(ClientResponse::Ok)
    }

    pub async fn handle_undo_request_message(
        &self,
        username: &PlayerUsername,
        game_id: &GameId,
        request: bool,
    ) -> ServiceResult<ClientResponse> {
        self.game_service
            .request_undo(username, game_id, request)
            .await?;
        Ok(ClientResponse::Ok)
    }

    pub async fn handle_draw_offer_message(
        &self,
        username: &PlayerUsername,
        game_id: &GameId,
        offer: bool,
    ) -> ServiceResult<ClientResponse> {
        self.game_service
            .offer_draw(username, game_id, offer)
            .await?;
        Ok(ClientResponse::Ok)
    }

    pub async fn handle_resign_message(
        &self,
        username: &PlayerUsername,
        game_id: &GameId,
    ) -> ServiceResult<ClientResponse> {
        self.game_service.resign_game(username, game_id).await?;
        Ok(ClientResponse::Ok)
    }
}
