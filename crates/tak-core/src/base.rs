use std::collections::HashMap;

use crate::{
    InvalidActionReason, InvalidPlaceReason, TakAction, TakBaseGameSettings, TakGameResult,
    TakOpening, TakPlayer, TakReserve, TakVariant, TakWinReason, board::TakBoard,
};

#[derive(Clone, Debug)]
pub struct TakOngoingBaseGame {
    pub settings: TakBaseGameSettings,
    pub board: TakBoard,
    pub current_player: TakPlayer,
    pub reserves: (TakReserve, TakReserve),
    pub board_hash_history: HashMap<String, u32>,
    pub ply_index: usize,
}

impl TakOngoingBaseGame {
    pub fn new(settings: &TakBaseGameSettings) -> Self {
        let board = TakBoard::new(settings.board_size);
        let reserves = (settings.reserve.clone(), settings.reserve.clone());
        TakOngoingBaseGame {
            settings: settings.clone(),
            board,
            current_player: TakPlayer::White,
            reserves,
            board_hash_history: HashMap::new(),
            ply_index: 0,
        }
    }

    pub fn can_do_action(&self, action: &TakAction) -> Result<(), InvalidActionReason> {
        let is_opening_action = self.ply_index < 2;
        let is_first_move = self.ply_index == 0;
        if is_opening_action {
            match action {
                TakAction::Place { pos, variant } => {
                    if *variant != TakVariant::Flat {
                        return Err(InvalidActionReason::OpeningViolation);
                    }
                    let (reserve, opponent_reserve) = match self.current_player {
                        TakPlayer::White => (&self.reserves.0, &self.reserves.1),
                        TakPlayer::Black => (&self.reserves.1, &self.reserves.0),
                    };
                    if !match self.settings.opening {
                        TakOpening::Swap => opponent_reserve.has(TakVariant::Flat, 1),
                        TakOpening::NoSwap => reserve.has(TakVariant::Flat, 1),
                        TakOpening::DoubleStack => opponent_reserve
                            .has(TakVariant::Flat, if is_first_move { 2 } else { 1 }),
                    } {
                        return Err(InvalidActionReason::InvalidPlace(
                            InvalidPlaceReason::NoPiecesRemaining,
                        ));
                    }
                    self.board
                        .can_do_place(pos)
                        .map_err(|e| InvalidActionReason::InvalidPlace(e))
                }
                TakAction::Move { .. } => Err(InvalidActionReason::OpeningViolation),
            }
        } else {
            match action {
                TakAction::Place { pos, variant } => {
                    let reserve = match self.current_player {
                        TakPlayer::White => &self.reserves.0,
                        TakPlayer::Black => &self.reserves.1,
                    };
                    if !reserve.has(*variant, 1) {
                        return Err(InvalidActionReason::InvalidPlace(
                            InvalidPlaceReason::NoPiecesRemaining,
                        ));
                    }
                    self.board
                        .can_do_place(pos)
                        .map_err(|e| InvalidActionReason::InvalidPlace(e))
                }
                TakAction::Move { pos, dir, drops } => self
                    .board
                    .can_do_move(pos, *dir, drops)
                    .map_err(|e| InvalidActionReason::InvalidMove(e)),
            }
        }
    }

    pub fn do_action(
        &mut self,
        action: TakAction,
    ) -> Result<Option<TakGameResult>, InvalidActionReason> {
        if let Err(e) = self.can_do_action(&action) {
            return Err(e);
        }
        self.ply_index += 1;
        let is_opening_action = self.ply_index < 2;
        let moved_player = self.current_player;
        let is_first_move = self.ply_index == 0;
        match &action {
            TakAction::Place { pos, variant } => {
                let (reserve, opponent_reserve) = match self.current_player {
                    TakPlayer::White => (&mut self.reserves.0, &mut self.reserves.1),
                    TakPlayer::Black => (&mut self.reserves.1, &mut self.reserves.0),
                };
                if is_opening_action {
                    match self.settings.opening {
                        TakOpening::Swap => opponent_reserve.try_take(*variant, 1),
                        TakOpening::NoSwap => reserve.try_take(*variant, 1),
                        TakOpening::DoubleStack => {
                            opponent_reserve.try_take(*variant, if is_first_move { 2 } else { 1 })
                        }
                    }
                    .expect(
                        "can_do_action should have prevented invalid place due to reserve state",
                    );
                } else {
                    reserve.try_take(*variant, 1).expect(
                        "can_do_action should have prevented invalid place due to reserve state",
                    );
                }
                let placing_composition = if is_opening_action {
                    match self.settings.opening {
                        TakOpening::Swap => vec![self.current_player.opponent()],
                        TakOpening::DoubleStack => {
                            vec![self.current_player.opponent(); if is_first_move { 2 } else { 1 }]
                        }
                        TakOpening::NoSwap => vec![self.current_player],
                    }
                } else {
                    vec![self.current_player]
                };
                self.board
                    .do_place(pos, *variant, placing_composition)
                    .expect("can_do_action should have prevented invalid place due to board state");
            }
            TakAction::Move { pos, dir, drops } => {
                self.board
                    .do_move(pos, *dir, drops)
                    .expect("can_do_action should have prevented invalid move due to board state");
            }
        }
        self.ply_index += 1;
        self.current_player = self.current_player.opponent();

        let board_hash = self.board.compute_hash_string();
        self.board_hash_history
            .entry(board_hash.clone())
            .and_modify(|e| *e += 1)
            .or_insert(1);

        Ok(self.check_game_over(board_hash, moved_player))
    }

    fn check_game_over(
        &self,
        board_hash: String,
        moved_player: TakPlayer,
    ) -> Option<TakGameResult> {
        let white_reserve_empty = self.reserves.0.pieces == 0 && self.reserves.0.capstones == 0;
        let black_reserve_empty = self.reserves.1.pieces == 0 && self.reserves.1.capstones == 0;

        let game_result = if self.board.check_for_road(moved_player) {
            Some(TakGameResult::Win {
                winner: moved_player,
                reason: TakWinReason::Road,
            })
        } else if self.board.check_for_road(moved_player.opponent()) {
            Some(TakGameResult::Win {
                winner: moved_player.opponent(),
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
        Some(game_result)
    }
}
