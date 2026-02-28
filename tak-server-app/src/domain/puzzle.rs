use std::collections::HashMap;

use tak_core::{TakAction, TakBaseGameSettings, ptn::TakGamePosition};

use crate::domain::{PuzzleId, RepoRetrieveError};

#[async_trait::async_trait]
pub trait PuzzleRepository {
    async fn get_puzzle(&self, id: PuzzleId) -> Result<Puzzle, RepoRetrieveError>;
}

#[derive(Clone, Debug)]
pub struct Puzzle {
    pub id: PuzzleId,
    pub game_settings: TakBaseGameSettings,
    pub position: TakGamePosition,
    pub responses: PuzzleResponseEntry,
}

impl Puzzle {
    pub fn do_response(&self, actions: &[TakAction]) -> PuzzleResponse {
        let mut current_node = &self.responses;
        for action in actions {
            match current_node.responses.get(action) {
                Some(Some(next_node)) => current_node = next_node,
                Some(None) => return PuzzleResponse::Success,
                None => return PuzzleResponse::Failure,
            }
        }
        PuzzleResponse::Response(current_node.action.clone())
    }
}

#[derive(Clone, Debug)]
pub struct PuzzleResponseEntry {
    action: TakAction,
    responses: HashMap<TakAction, Option<PuzzleResponseEntry>>,
}

impl PuzzleResponseEntry {
    pub fn new(
        action: TakAction,
        responses: HashMap<TakAction, Option<PuzzleResponseEntry>>,
    ) -> Self {
        Self { action, responses }
    }
}

#[derive(Clone, Debug)]
pub enum PuzzleResponse {
    Success,
    Response(TakAction),
    Failure,
}
