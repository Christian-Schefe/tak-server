use crate::tak::{
    TakAction, TakGameSettings, TakGameState, TakPlayer, TakVariant, TakWinReason, board::TakBoard,
};

#[derive(Clone)]
pub struct TakGame {
    pub settings: TakGameSettings,
    pub board: TakBoard,
    pub current_player: TakPlayer,
    pub reserves: (TakReserve, TakReserve),
    pub move_history: Vec<TakAction>,
    pub game_state: TakGameState,
}

#[derive(Clone)]
pub struct TakReserve {
    pub pieces: u32,
    pub capstones: u32,
}

impl TakGame {
    pub fn new(settings: TakGameSettings) -> Self {
        let board = TakBoard::new(settings.board_size);
        let reserve = TakReserve {
            pieces: settings.reserve_pieces,
            capstones: settings.reserve_capstones,
        };
        let reserves = (reserve.clone(), reserve);
        TakGame {
            settings,
            board,
            current_player: TakPlayer::White,
            reserves,
            move_history: Vec::new(),
            game_state: TakGameState::Ongoing,
        }
    }

    pub fn can_do_action(&self, action: &TakAction) -> Result<(), String> {
        match action {
            TakAction::Place { pos, variant } => {
                self.board.can_do_place(pos)?;
                if self.move_history.len() < 2 && *variant != TakVariant::Flat {
                    return Err("First two moves must be flat stones".to_string());
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
                    return Err("No pieces of the specified variant left in reserve".to_string());
                }
                Ok(())
            }
            TakAction::Move { pos, dir, drops } => {
                self.board.can_do_move(pos, dir, drops)?;
                if self.move_history.len() < 2 {
                    return Err("Cannot move pieces before both players have placed at least one piece each".to_string());
                }
                Ok(())
            }
        }
    }

    pub fn do_action(&mut self, action: &TakAction) -> Result<(), String> {
        self.can_do_action(&action)?;
        match &action {
            TakAction::Place { pos, variant } => {
                let placing_player = if self.move_history.len() < 2 {
                    self.current_player.opponent()
                } else {
                    self.current_player.clone()
                };
                self.board.do_place(pos, variant, &placing_player)?;
                let reserve = match self.current_player {
                    TakPlayer::White => &mut self.reserves.0,
                    TakPlayer::Black => &mut self.reserves.1,
                };
                let amount = match variant {
                    TakVariant::Flat | TakVariant::Standing => &mut reserve.pieces,
                    TakVariant::Capstone => &mut reserve.capstones,
                };
                *amount -= 1;
            }
            TakAction::Move { pos, dir, drops } => {
                self.board.do_move(pos, dir, drops)?;
            }
        }

        let white_reserve_empty = self.reserves.0.pieces == 0 && self.reserves.0.capstones == 0;
        let black_reserve_empty = self.reserves.1.pieces == 0 && self.reserves.1.capstones == 0;

        if self.board.check_for_road(&self.current_player) {
            self.game_state = TakGameState::Win {
                winner: self.current_player.clone(),
                reason: TakWinReason::Road,
            };
        } else if self.board.check_for_road(&self.current_player.opponent()) {
            self.game_state = TakGameState::Win {
                winner: self.current_player.opponent(),
                reason: TakWinReason::Road,
            };
        } else if self.board.is_full() || white_reserve_empty || black_reserve_empty {
            let (white_flats, black_flats) = self.board.count_flats();
            let (white_score, black_score) =
                (white_flats * 2, black_flats * 2 + self.settings.half_komi);
            self.game_state = match white_score.cmp(&black_score) {
                std::cmp::Ordering::Greater => TakGameState::Win {
                    winner: TakPlayer::White,
                    reason: TakWinReason::Flats,
                },
                std::cmp::Ordering::Less => TakGameState::Win {
                    winner: TakPlayer::Black,
                    reason: TakWinReason::Flats,
                },
                std::cmp::Ordering::Equal => TakGameState::Draw,
            };
        }

        self.move_history.push(action.clone());
        self.current_player = self.current_player.opponent();

        Ok(())
    }

    pub fn resign(&mut self, player: &TakPlayer) {
        self.game_state = TakGameState::Win {
            winner: player.opponent(),
            reason: TakWinReason::Default,
        };
    }
}
