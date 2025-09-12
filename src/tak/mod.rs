mod board;
mod game;

pub use game::TakGame;

#[derive(Clone, PartialEq)]
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

#[derive(Clone)]
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

#[derive(Clone)]
pub struct TakGameSettings {
    pub board_size: u32,
    pub half_komi: u32,
    pub reserve_pieces: u32,
    pub reserve_capstones: u32,
    pub time_contingent_seconds: u32,
    pub time_increment_seconds: u32,
    pub time_extra: Option<TimeExtra>,
}

impl TakGameSettings {
    pub fn is_valid(&self) -> bool {
        self.board_size >= 3
            && self.board_size <= 8
            && self.reserve_pieces > 0
            && self.time_contingent_seconds > 0
    }
}

#[derive(Clone)]
pub struct TimeExtra {
    pub trigger_move: u32,
    pub extra_seconds: u32,
}

#[derive(Clone, PartialEq)]
pub enum TakVariant {
    Flat,
    Standing,
    Capstone,
}

#[derive(Clone)]
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

#[derive(Clone)]
pub enum TakDir {
    Up,
    Left,
    Right,
    Down,
}

#[derive(Clone, PartialEq)]
pub enum TakGameState {
    Ongoing,
    Win {
        winner: TakPlayer,
        reason: TakWinReason,
    },
    Draw,
}

#[derive(Clone, PartialEq)]
pub enum TakWinReason {
    Road,
    Flats,
    Default,
}
