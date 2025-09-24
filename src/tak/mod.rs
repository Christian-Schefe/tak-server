mod board;
mod game;
pub mod ptn;

use std::time::Duration;

pub use game::TakGame;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

#[derive(Clone, Debug)]
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
pub struct TakGameSettings {
    pub board_size: u32,
    pub half_komi: u32,
    pub reserve_pieces: u32,
    pub reserve_capstones: u32,
    pub time_control: TakTimeControl,
}

impl TakGameSettings {
    pub fn is_valid(&self) -> bool {
        self.board_size >= 3
            && self.board_size <= 8
            && self.reserve_pieces > 0
            && !self.time_control.contingent.is_zero()
            && self
                .time_control
                .extra
                .is_none_or(|(n, d)| n > 0 && !d.is_zero())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TakTimeControl {
    pub contingent: Duration,
    pub increment: Duration,
    pub extra: Option<(u32, Duration)>,
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

#[derive(Clone, Debug)]
pub enum TakDir {
    Up,
    Left,
    Right,
    Down,
}

#[derive(Clone, PartialEq, Debug)]
pub enum TakGameState {
    Ongoing,
    Win {
        winner: TakPlayer,
        reason: TakWinReason,
    },
    Draw,
}

impl TakGameState {
    pub fn to_string(&self) -> String {
        match self {
            TakGameState::Ongoing => "0-0".to_string(),
            TakGameState::Win { winner, reason } => match (winner, reason) {
                (TakPlayer::White, TakWinReason::Road) => "R-0".to_string(),
                (TakPlayer::White, TakWinReason::Flats) => "F-0".to_string(),
                (TakPlayer::White, TakWinReason::Default) => "1-0".to_string(),
                (TakPlayer::Black, TakWinReason::Road) => "0-R".to_string(),
                (TakPlayer::Black, TakWinReason::Flats) => "0-F".to_string(),
                (TakPlayer::Black, TakWinReason::Default) => "0-1".to_string(),
            },
            TakGameState::Draw => "1/2-1/2".to_string(),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum TakWinReason {
    Road,
    Flats,
    Default,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_tak_game_settings_validation() {
        let valid_settings = TakGameSettings {
            board_size: 5,
            half_komi: 0,
            reserve_pieces: 21,
            reserve_capstones: 1,
            time_control: TakTimeControl {
                contingent: Duration::from_secs(300),
                increment: Duration::from_secs(5),
                extra: Some((2, Duration::from_secs(60))),
            },
        };
        assert!(valid_settings.is_valid());

        let invalid_settings = TakGameSettings {
            board_size: 9,
            half_komi: 0,
            reserve_pieces: 21,
            reserve_capstones: 1,
            time_control: TakTimeControl {
                contingent: Duration::from_secs(300),
                increment: Duration::from_secs(5),
                extra: Some((2, Duration::from_secs(60))),
            },
        };
        assert!(!invalid_settings.is_valid());

        let invalid_time_control = TakGameSettings {
            board_size: 5,
            half_komi: 0,
            reserve_pieces: 21,
            reserve_capstones: 1,
            time_control: TakTimeControl {
                contingent: Duration::from_secs(0),
                increment: Duration::from_secs(5),
                extra: Some((2, Duration::from_secs(60))),
            },
        };
        assert!(!invalid_time_control.is_valid());

        let invalid_extra_time = TakGameSettings {
            board_size: 5,
            half_komi: 0,
            reserve_pieces: 21,
            reserve_capstones: 1,
            time_control: TakTimeControl {
                contingent: Duration::from_secs(300),
                increment: Duration::from_secs(5),
                extra: Some((0, Duration::from_secs(60))),
            },
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
