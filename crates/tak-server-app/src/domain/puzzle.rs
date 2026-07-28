use std::collections::HashMap;

use tak_core::{TakAction, TakBaseGameSettings};

use crate::domain::{PuzzleId, RepoError, RepoRetrieveError};

#[async_trait::async_trait]
pub trait PuzzleRepository {
    async fn get_puzzle(&self, id: PuzzleId) -> Result<Puzzle, RepoRetrieveError>;
    async fn select_random_puzzle(&self) -> Result<PuzzleId, RepoError>;
}

#[derive(Clone, Debug)]
pub struct Puzzle {
    pub id: PuzzleId,
    pub game_settings: TakBaseGameSettings,
    pub position: Vec<TakAction>,
    pub responses: PuzzleResponseEntry,
}

impl Puzzle {
    pub fn do_response(&self, actions: &[TakAction]) -> Option<PuzzleResponse> {
        let mut response_move = None;
        let mut current_node = &self.responses;
        for action in actions {
            match current_node.0.get(action) {
                Some(Some((action, next_node))) => {
                    current_node = next_node;
                    response_move = Some(action);
                }
                Some(None) => return Some(PuzzleResponse::Success),
                None => return Some(PuzzleResponse::Failure),
            }
        }
        response_move.cloned().map(PuzzleResponse::Response)
    }
}

#[derive(Clone, Debug)]
pub struct PuzzleResponseEntry(pub HashMap<TakAction, Option<(TakAction, PuzzleResponseEntry)>>);

#[derive(Clone, Debug)]
pub enum PuzzleResponse {
    Success,
    Response(TakAction),
    Failure,
}
