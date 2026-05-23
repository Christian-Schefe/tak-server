use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tak_core::{TakBaseGameSettings, TakOngoingBaseGame, TakPlayer};
use tak_server_api_contract::game::JsonGameStatus;

pub struct GameService {
    games: Arc<Mutex<HashMap<String, (TakPlayer, TakOngoingBaseGame)>>>,
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
        status: &JsonGameStatus,
    ) -> Option<(TakPlayer, TakOngoingBaseGame)> {
        let mut game = TakOngoingBaseGame::new(status.game_settings.to_game_settings().base);
        for action in &status.actions {
            let action = tak_core::ptn::action_from_ptn(action)?;
            if let Some(_finished_game) = game.do_action(action).ok()? {
                return None;
            }
        }
        self.games
            .lock()
            .unwrap()
            .insert(game_id.to_string(), (player, game.clone()));
        if game.current_player != player {
            return None;
        }
        Some((player, game))
    }

    pub fn begin_game(
        &self,
        game_id: String,
        player: TakPlayer,
        settings: TakBaseGameSettings,
    ) -> Option<(TakPlayer, TakOngoingBaseGame)> {
        let game = TakOngoingBaseGame::new(settings);
        self.games
            .lock()
            .unwrap()
            .insert(game_id, (player, game.clone()));
        if game.current_player != player {
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
    ) -> Option<(TakPlayer, TakOngoingBaseGame)> {
        let mut games = self.games.lock().unwrap();
        let game = games.get_mut(&game_id)?;
        if game.1.action_history.len() + 1 != ply_index {
            return None;
        }
        let action = tak_core::ptn::action_from_ptn(ptn_move)?;
        if let Some(_finished_game) = game.1.do_action(action).ok()? {
            games.remove(&game_id);
            return None;
        }
        if game.1.current_player != game.0 {
            return None;
        }
        Some(game.clone())
    }

    pub fn undo_action(
        &self,
        game_id: String,
        ply_index: usize,
    ) -> Option<(TakPlayer, TakOngoingBaseGame)> {
        let mut games = self.games.lock().unwrap();
        let game = games.get_mut(&game_id)?;
        if game.1.action_history.len() - 1 != ply_index {
            return None;
        }
        if !game.1.undo_action() {
            return None;
        }
        if game.1.current_player != game.0 {
            return None;
        }
        Some(game.clone())
    }
}
