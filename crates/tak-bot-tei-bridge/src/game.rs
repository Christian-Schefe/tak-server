use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tak_core::{TakGame, TakGameSettings, TakPlayer};
use tak_server_api_contract::game::JsonGameDetail;

pub struct GameService {
    games: Arc<Mutex<HashMap<String, (TakPlayer, TakGame<()>)>>>,
}

impl GameService {
    pub fn new() -> Self {
        Self {
            games: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn load_game(
        &self,
        game_id: String,
        player: TakPlayer,
        status: &JsonGameDetail,
    ) -> Option<(TakPlayer, TakGame<()>)> {
        let mut game = TakGame::new(&status.record.metadata.game_settings.to_game_settings(), ());
        for action in &status.record.history {
            let action = tak_core::ptn::action_from_ptn(action)?;
            match game.do_action(action, ()) {
                Ok(()) => {}
                Err(_) => return None,
            }
        }
        self.games
            .lock()
            .unwrap()
            .insert(game_id.to_string(), (player, game.clone()));
        if game.current_player() != player {
            return None;
        }
        Some((player, game))
    }

    pub fn begin_game(
        &self,
        game_id: String,
        player: TakPlayer,
        settings: &TakGameSettings,
    ) -> Option<(TakPlayer, TakGame<()>)> {
        let game = TakGame::new(settings, ());
        self.games
            .lock()
            .unwrap()
            .insert(game_id, (player, game.clone()));
        if game.current_player() != player {
            return None;
        }
        Some((player, game))
    }

    pub fn end_game(&self, game_id: String) {
        self.games.lock().unwrap().remove(&game_id);
    }

    pub fn do_action(
        &self,
        game_id: String,
        ptn_move: &str,
        ply_index: usize,
    ) -> Option<(TakPlayer, TakGame<()>)> {
        let mut games = self.games.lock().unwrap();
        let game = games.get_mut(&game_id)?;
        if game.1.ply_index() + 1 != ply_index {
            return None;
        }
        let action = tak_core::ptn::action_from_ptn(ptn_move)?;

        if let Err(_) = game.1.do_action(action, ()) {
            games.remove(&game_id);
            return None;
        }
        if !game.1.is_ongoing() {
            games.remove(&game_id);
            return None;
        }
        if game.1.current_player() != game.0 {
            return None;
        }
        Some(game.clone())
    }

    pub fn undo_action(
        &self,
        game_id: String,
        ply_index: usize,
    ) -> Option<(TakPlayer, TakGame<()>)> {
        let mut games = self.games.lock().unwrap();
        let game = games.get_mut(&game_id)?;
        if game.1.ply_index() - 1 != ply_index {
            return None;
        }
        if let Ok(false) = game.1.try_undo_action(()) {
            return None;
        }
        if game.1.current_player() != game.0 {
            return None;
        }
        Some(game.clone())
    }
}
