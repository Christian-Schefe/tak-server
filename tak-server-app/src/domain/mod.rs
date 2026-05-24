pub mod chat;
pub mod event;
pub mod game;
pub mod game_history;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayerId(pub uuid::Uuid);

impl std::fmt::Display for PlayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_hyphenated())
    }
}
impl TryFrom<String> for PlayerId {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if let Ok(uuid) = uuid::Uuid::parse_str(&value) {
            Ok(PlayerId(uuid))
        } else {
            Err(format!("Failed to parse PlayerId from string: {}", value))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AccountId(pub uuid::Uuid);

impl AccountId {
    pub fn new() -> Self {
        AccountId(uuid::Uuid::new_v4())
    }
}

impl TryFrom<String> for AccountId {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if let Ok(uuid) = uuid::Uuid::parse_str(&value) {
            Ok(AccountId(uuid))
        } else {
            Err(format!("Failed to parse AccountId from string: {}", value))
        }
    }
}

impl std::fmt::Display for AccountId {
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

#[derive(Debug, Clone)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Default)]
pub struct Pagination {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

impl Pagination {
    pub fn new(page: usize, page_size: usize) -> Self {
        let offset = Some((page - 1) * page_size);
        let limit = Some(page_size);
        Self { offset, limit }
    }
}

pub struct PaginatedResponse<T> {
    pub total_count: usize,
    pub items: Vec<T>,
}

impl<T> PaginatedResponse<T> {
    pub fn map<U, F>(self, f: F) -> PaginatedResponse<U>
    where
        F: FnMut(T) -> U,
    {
        PaginatedResponse {
            total_count: self.total_count,
            items: self.items.into_iter().map(f).collect(),
        }
    }
}

#[derive(Debug)]
pub enum RepoError {
    StorageError(String),
}

impl std::fmt::Display for RepoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepoError::StorageError(e) => write!(f, "Storage error: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum RepoRetrieveError {
    NotFound,
    StorageError(String),
}

impl std::fmt::Display for RepoRetrieveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepoRetrieveError::NotFound => write!(f, "Resource not found"),
            RepoRetrieveError::StorageError(e) => write!(f, "Storage error: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum RepoCreateError {
    Conflict,
    StorageError(String),
}

impl std::fmt::Display for RepoCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepoCreateError::Conflict => write!(f, "Resource conflict"),
            RepoCreateError::StorageError(e) => write!(f, "Storage error: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum RepoUpdateError {
    NotFound,
    Conflict,
    StorageError(String),
}

impl std::fmt::Display for RepoUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepoUpdateError::NotFound => write!(f, "Resource not found"),
            RepoUpdateError::Conflict => write!(f, "Resource conflict"),
            RepoUpdateError::StorageError(e) => write!(f, "Storage error: {}", e),
        }
    }
}
