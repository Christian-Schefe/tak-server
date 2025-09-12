use crate::{
    client::ClientId,
    game::Game,
    protocol::{BoxedProtocolHandler, ProtocolHandler},
    seek::Seek,
    tak::TakAction,
};

pub struct ProtocolJSONHandler(ClientId);

impl ProtocolHandler for ProtocolJSONHandler {
    fn new(id: ClientId) -> Self
    where
        Self: Sized,
    {
        ProtocolJSONHandler(id)
    }

    fn clone_box(&self) -> BoxedProtocolHandler {
        Box::new(ProtocolJSONHandler(self.0))
    }

    fn get_client_id(&self) -> ClientId {
        self.0
    }

    fn handle_message(&self, msg: String) {
        todo!()
    }

    fn send_new_seek_message(&self, seek: &Seek) {
        todo!()
    }

    fn send_remove_seek_message(&self, seek: &Seek) {
        todo!()
    }

    fn send_new_game_message(&self, game: &Game) {
        todo!()
    }

    fn send_remove_game_message(&self, game: &Game) {
        todo!()
    }

    fn send_game_start_message(&self, game: &Game) {
        todo!()
    }

    fn send_game_action_message(&self, game: &Game, action: &TakAction) {
        todo!()
    }

    fn send_game_over_message(&self, game: &Game) {
        todo!()
    }
}
