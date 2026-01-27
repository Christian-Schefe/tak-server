mod base;
mod board;
mod game;
pub mod ptn;

use std::time::Duration;

pub use game::TakFinishedGame;
pub use game::TakOngoingGame;

#[derive(Clone, Debug, PartialEq)]
pub struct TakGameSettings {
    pub base: TakBaseGameSettings,
    pub time_settings: TakTimeSettings,
}

impl TakGameSettings {
    pub fn is_valid(&self) -> bool {
        self.base.is_valid()
            && match &self.time_settings {
                TakTimeSettings::Realtime(rt) => rt.is_valid(),
                TakTimeSettings::Async(at) => at.is_valid(),
            }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TakTimeSettings {
    Realtime(TakRealtimeTimeControl),
    Async(TakAsyncTimeControl),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TakTimeInfo {
    pub white_remaining: Duration,
    pub black_remaining: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TakPlayer {
    White,
    Black,
}

impl TakPlayer {
    pub fn opponent(&self) -> TakPlayer {
        match self {
            TakPlayer::White => TakPlayer::Black,
            TakPlayer::Black => TakPlayer::White,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TakAction {
    Place {
        pos: TakPos,
        variant: TakVariant,
    },
    Move {
        pos: TakPos,
        dir: TakDir,
        drops: Vec<u32>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TakReserve {
    pub pieces: u32,
    pub capstones: u32,
}

impl TakReserve {
    pub fn new(pieces: u32, capstones: u32) -> Self {
        TakReserve { pieces, capstones }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TakBaseGameSettings {
    pub board_size: u32,
    pub half_komi: u32,
    pub reserve: TakReserve,
}

impl TakBaseGameSettings {
    pub fn is_valid(&self) -> bool {
        self.board_size >= 3 && self.board_size <= 8 && self.reserve.pieces > 0
    }
}

impl TakRealtimeTimeControl {
    pub fn is_valid(&self) -> bool {
        !self.contingent.is_zero() && self.extra.is_none_or(|(n, d)| n > 0 && !d.is_zero())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TakRealtimeTimeControl {
    pub contingent: Duration,
    pub increment: Duration,
    pub extra: Option<(u32, Duration)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TakAsyncTimeControl {
    pub contingent: Duration,
}

impl TakAsyncTimeControl {
    pub fn is_valid(&self) -> bool {
        !self.contingent.is_zero()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TakVariant {
    Flat,
    Standing,
    Capstone,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TakPos {
    pub x: i32,
    pub y: i32,
}

impl TakPos {
    pub fn new(x: i32, y: i32) -> Self {
        TakPos { x, y }
    }

    pub fn is_valid(&self, size: u32) -> bool {
        self.x >= 0 && self.x < size as i32 && self.y >= 0 && self.y < size as i32
    }

    pub fn offset(&self, dir: &TakDir, distance: i32) -> Self {
        match dir {
            TakDir::Up => TakPos {
                x: self.x,
                y: self.y + distance,
            },
            TakDir::Right => TakPos {
                x: self.x + distance,
                y: self.y,
            },
            TakDir::Down => TakPos {
                x: self.x,
                y: self.y - distance,
            },
            TakDir::Left => TakPos {
                x: self.x - distance,
                y: self.y,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TakDir {
    Up,
    Left,
    Right,
    Down,
}

#[derive(Clone, PartialEq, Debug)]
pub enum TakGameResult {
    Win {
        winner: TakPlayer,
        reason: TakWinReason,
    },
    Draw,
}

#[derive(Clone, PartialEq, Debug)]
pub enum TakWinReason {
    Road,
    Flats,
    Default,
}

impl TakWinReason {
    pub const ALL: [TakWinReason; 3] = [
        TakWinReason::Road,
        TakWinReason::Flats,
        TakWinReason::Default,
    ];
}

pub enum MaybeTimeout<R, T> {
    Result(R),
    Timeout(T),
}

#[derive(Clone, Debug)]
pub enum InvalidActionReason {
    OpeningViolation,
    InvalidPlace(InvalidPlaceReason),
    InvalidMove(InvalidMoveReason),
}

#[derive(Clone, Debug)]
pub enum InvalidPlaceReason {
    OutOfBounds,
    PositionOccupied,
    NoPiecesRemaining,
}

#[derive(Clone, Debug)]
pub enum InvalidMoveReason {
    OutOfBounds,
    PositionEmpty,
    InvalidNumberOfPieces,
    CannotMoveOverStandingPieces,
    CannotMoveOverCapstonePieces,
    InvalidDropDistribution,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_tak_game_settings_validation() {
        let valid_settings = TakGameSettings {
            base: TakBaseGameSettings {
                board_size: 5,
                half_komi: 0,
                reserve: TakReserve::new(21, 1),
            },
            time_settings: TakTimeSettings::Realtime(TakRealtimeTimeControl {
                contingent: Duration::from_secs(300),
                increment: Duration::from_secs(5),
                extra: Some((2, Duration::from_secs(60))),
            }),
        };
        assert!(valid_settings.is_valid());

        let invalid_settings = TakGameSettings {
            base: TakBaseGameSettings {
                board_size: 9,
                half_komi: 0,
                reserve: TakReserve::new(21, 1),
            },
            time_settings: TakTimeSettings::Realtime(TakRealtimeTimeControl {
                contingent: Duration::from_secs(300),
                increment: Duration::from_secs(5),
                extra: Some((2, Duration::from_secs(60))),
            }),
        };
        assert!(!invalid_settings.is_valid());

        let invalid_time_control = TakGameSettings {
            base: TakBaseGameSettings {
                board_size: 5,
                half_komi: 0,
                reserve: TakReserve::new(21, 1),
            },
            time_settings: TakTimeSettings::Realtime(TakRealtimeTimeControl {
                contingent: Duration::from_secs(0),
                increment: Duration::from_secs(5),
                extra: Some((2, Duration::from_secs(60))),
            }),
        };
        assert!(!invalid_time_control.is_valid());

        let invalid_extra_time = TakGameSettings {
            base: TakBaseGameSettings {
                board_size: 5,
                half_komi: 0,
                reserve: TakReserve::new(21, 1),
            },
            time_settings: TakTimeSettings::Realtime(TakRealtimeTimeControl {
                contingent: Duration::from_secs(300),
                increment: Duration::from_secs(5),
                extra: Some((0, Duration::from_secs(60))),
            }),
        };
        assert!(!invalid_extra_time.is_valid());
    }

    #[test]
    fn test_tak_pos_validation() {
        let valid_pos = TakPos::new(3, 3);
        assert!(valid_pos.is_valid(5));

        let invalid_pos_x = TakPos::new(-1, 3);
        assert!(!invalid_pos_x.is_valid(5));

        let invalid_pos_y = TakPos::new(3, -1);
        assert!(!invalid_pos_y.is_valid(5));

        let out_of_bounds_pos = TakPos::new(5, 5);
        assert!(!out_of_bounds_pos.is_valid(5));
    }

    #[test]
    fn test_tak_pos_offset() {
        let start_pos = TakPos::new(2, 2);

        let up_pos = start_pos.offset(&TakDir::Up, 1);
        assert_eq!(up_pos, TakPos::new(2, 3));

        let down_pos = start_pos.offset(&TakDir::Down, 1);
        assert_eq!(down_pos, TakPos::new(2, 1));

        let left_pos = start_pos.offset(&TakDir::Left, 1);
        assert_eq!(left_pos, TakPos::new(1, 2));

        let right_pos = start_pos.offset(&TakDir::Right, 1);
        assert_eq!(right_pos, TakPos::new(3, 2));
    }
}
