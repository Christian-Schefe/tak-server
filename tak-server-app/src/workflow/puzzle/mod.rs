use tak_core::{TakBaseGameSettings, ptn::TakGamePosition};

use crate::domain::{PuzzleId, puzzle::Puzzle};

pub mod get;
pub mod solve;

#[derive(Clone, Debug)]
pub struct PuzzleView {
    pub id: PuzzleId,
    pub position: TakGamePosition,
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
