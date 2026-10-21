pub mod chat;
pub mod event;
pub mod game;
pub mod matches;
pub mod moderation;
pub mod player;
pub mod profile;
pub mod puzzle;
pub mod rating;
pub mod seek;
pub mod spectator;
pub mod stats;
pub mod tournament;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UserId(pub uuid::Uuid);

impl UserId {
    pub fn new() -> Self {
        UserId(uuid::Uuid::new_v4())
    }
}

impl TryFrom<String> for UserId {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if let Ok(uuid) = uuid::Uuid::parse_str(&value) {
            Ok(UserId(uuid))
        } else {
            Err(format!("Failed to parse UserId from string: {}", value))
        }
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_hyphenated())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MatchId(pub i64);

impl std::fmt::Display for MatchId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for MatchId {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value
            .parse::<i64>()
            .map(MatchId)
            .map_err(|e| format!("Failed to parse MatchId from string: {}", e))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SeekId(pub u64);

impl SeekId {
    pub fn new(id: u64) -> Self {
        SeekId(id)
    }
}

impl std::fmt::Display for SeekId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChatMessageId(pub i64);

impl ChatMessageId {
    pub fn new(id: i64) -> Self {
        ChatMessageId(id)
    }
}

impl std::fmt::Display for ChatMessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PuzzleId(pub i64);

impl PuzzleId {
    pub fn new(id: i64) -> Self {
        PuzzleId(id)
    }
}

impl std::fmt::Display for PuzzleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GameId(pub i64);

impl GameId {
    pub fn new(id: i64) -> Self {
        GameId(id)
    }
}

impl TryFrom<String> for GameId {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value
            .parse::<i64>()
            .map(GameId)
            .map_err(|e| format!("Failed to parse GameId from string: {}", e))
    }
}

impl std::fmt::Display for GameId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TournamentId(pub i64);

impl TournamentId {
    pub fn new(id: i64) -> Self {
        TournamentId(id)
    }
}

impl std::fmt::Display for TournamentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ListenerId(uuid::Uuid);

impl ListenerId {
    pub fn new() -> Self {
        ListenerId(uuid::Uuid::new_v4())
    }
}

impl std::fmt::Display for ListenerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_hyphenated())
    }
}
