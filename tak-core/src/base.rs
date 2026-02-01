use std::collections::HashMap;

use crate::{
    InvalidActionReason, InvalidPlaceReason, TakAction, TakGameResult, TakBaseGameSettings, TakPlayer,
    TakReserve, TakVariant, TakWinReason, board::TakBoard,
};

#[derive(Clone, Debug)]
pub struct TakFinishedBaseGame {
    pub game_result: TakGameResult,
    pub action_history: Vec<TakAction>,
}

impl TakFinishedBaseGame {
    pub fn new(ended_game: &TakOngoingBaseGame, game_result: TakGameResult) -> Self {
        TakFinishedBaseGame {
            game_result,
            action_history: ended_game.action_history.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TakOngoingBaseGame {
    pub settings: TakBaseGameSettings,
    pub board: TakBoard,
    pub current_player: TakPlayer,
    pub reserves: (TakReserve, TakReserve),
    pub board_hash_history: HashMap<String, u32>,
    pub action_history: Vec<TakAction>,
}

impl TakOngoingBaseGame {
    pub fn new(settings: TakBaseGameSettings) -> Self {
        let board = TakBoard::new(settings.board_size);
        let reserves = (settings.reserve.clone(), settings.reserve.clone());
        TakOngoingBaseGame {
            settings,
            board,
            current_player: TakPlayer::White,
            reserves,
            board_hash_history: HashMap::new(),
            action_history: Vec::new(),
        }
    }

    pub fn can_do_action(&self, action: &TakAction) -> Result<(), InvalidActionReason> {
        match action {
            TakAction::Place { pos, variant } => {
                if self.action_history.len() < 2 && *variant != TakVariant::Flat {
                    return Err(InvalidActionReason::OpeningViolation);
                }
                let reserve = match self.current_player {
                    TakPlayer::White => &self.reserves.0,
                    TakPlayer::Black => &self.reserves.1,
                };
                let amount = match variant {
                    TakVariant::Flat | TakVariant::Standing => reserve.pieces,
                    TakVariant::Capstone => reserve.capstones,
                };
                if amount == 0 {
                    return Err(InvalidActionReason::InvalidPlace(
                        InvalidPlaceReason::NoPiecesRemaining,
                    ));
                }
                if let Err(e) = self.board.can_do_place(pos) {
                    return Err(InvalidActionReason::InvalidPlace(e));
                }
                Ok(())
            }
            TakAction::Move { pos, dir, drops } => {
                if self.action_history.len() < 2 {
                    return Err(InvalidActionReason::OpeningViolation);
                }
                if let Err(e) = self.board.can_do_move(pos, dir, drops) {
                    return Err(InvalidActionReason::InvalidMove(e));
                }
                Ok(())
            }
        }
    }

    pub fn do_action(
        &mut self,
        action: TakAction,
    ) -> Result<Option<TakFinishedBaseGame>, InvalidActionReason> {
        if let Err(e) = self.can_do_action(&action) {
            return Err(e);
        }
        match &action {
            TakAction::Place { pos, variant } => {
                let placing_player = if self.action_history.len() < 2 {
                    self.current_player.opponent()
                } else {
                    self.current_player.clone()
                };
                let reserve = match self.current_player {
                    TakPlayer::White => &mut self.reserves.0,
                    TakPlayer::Black => &mut self.reserves.1,
                };
                let amount = match variant {
                    TakVariant::Flat | TakVariant::Standing => &mut reserve.pieces,
                    TakVariant::Capstone => &mut reserve.capstones,
                };
                *amount -= 1;
                self.board
                    .do_place(pos, variant, &placing_player)
                    .expect("can_do_action should have prevented invalid place due to board state");
            }
            TakAction::Move { pos, dir, drops } => {
                self.board
                    .do_move(pos, dir, drops)
                    .expect("can_do_action should have prevented invalid move due to board state");
            }
        }

        let board_hash = self.board.compute_hash_string();
        self.board_hash_history
            .entry(board_hash.clone())
            .and_modify(|e| *e += 1)
            .or_insert(1);

        match self.check_game_over(board_hash) {
            Some(finished_game) => {
                return Ok(Some(finished_game));
            }
            None => {
                self.current_player = self.current_player.opponent();
                self.action_history.push(action);

                Ok(None)
            }
        }
    }

    pub fn check_game_over(&self, board_hash: String) -> Option<TakFinishedBaseGame> {
        let white_reserve_empty = self.reserves.0.pieces == 0 && self.reserves.0.capstones == 0;
        let black_reserve_empty = self.reserves.1.pieces == 0 && self.reserves.1.capstones == 0;

        let game_result = if self.board.check_for_road(&self.current_player) {
            Some(TakGameResult::Win {
                winner: self.current_player.clone(),
                reason: TakWinReason::Road,
            })
        } else if self.board.check_for_road(&self.current_player.opponent()) {
            Some(TakGameResult::Win {
                winner: self.current_player.opponent(),
                reason: TakWinReason::Road,
            })
        } else if self.board.is_full() || white_reserve_empty || black_reserve_empty {
            let (white_flats, black_flats) = self.board.count_flats();
            let (white_score, black_score) =
                (white_flats * 2, black_flats * 2 + self.settings.half_komi);
            Some(match white_score.cmp(&black_score) {
                std::cmp::Ordering::Greater => TakGameResult::Win {
                    winner: TakPlayer::White,
                    reason: TakWinReason::Flats,
                },
                std::cmp::Ordering::Less => TakGameResult::Win {
                    winner: TakPlayer::Black,
                    reason: TakWinReason::Flats,
                },
                std::cmp::Ordering::Equal => TakGameResult::Draw,
            })
        } else if let Some(repeat_count) = self.board_hash_history.get(&board_hash)
            && *repeat_count >= 3
        {
            Some(TakGameResult::Draw)
        } else {
            None
        }?;
        Some(TakFinishedBaseGame::new(self, game_result))
    }
}
