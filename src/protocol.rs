mod json;
mod v2;

pub use json::ProtocolJSONHandler;
pub use v2::ProtocolV2Handler;

use crate::{client::ClientId, game::Game, seek::Seek, tak::TakAction};

pub trait ProtocolHandler {
    fn new(id: ClientId) -> Self
    where
        Self: Sized;
    fn get_client_id(&self) -> ClientId;
    fn clone_box(&self) -> BoxedProtocolHandler;

    fn handle_message(&self, msg: String);

    fn send_new_seek_message(&self, seek: &Seek);
    fn send_remove_seek_message(&self, seek: &Seek);

    fn send_new_game_message(&self, game: &Game);
    fn send_remove_game_message(&self, game: &Game);

    fn send_game_start_message(&self, game: &Game);
    fn send_game_action_message(&self, game: &Game, action: &TakAction);
    fn send_game_over_message(&self, game: &Game);
}

pub type BoxedProtocolHandler = Box<dyn ProtocolHandler + Send + Sync>;
