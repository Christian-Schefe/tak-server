use tak_core::{TakAction, TakBaseGameSettings};

use crate::domain::{PuzzleId, puzzle::Puzzle};

pub mod get;
pub mod solve;

#[derive(Clone, Debug)]
pub struct PuzzleView {
    pub id: PuzzleId,
    pub position: Vec<TakAction>,
    pub game_settings: TakBaseGameSettings,
}

impl PuzzleView {
    pub fn from(puzzle: &Puzzle) -> Self {
        Self {
            id: puzzle.id,
            position: puzzle.position.clone(),
            game_settings: puzzle.game_settings.clone(),
        }
    }
}
