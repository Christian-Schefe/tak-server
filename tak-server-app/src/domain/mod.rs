pub mod account;
pub mod chat;
pub mod event;
pub mod game;
pub mod game_history;
pub mod r#match;
pub mod player;
pub mod rating;
pub mod seek;
pub mod spectator;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayerId(pub uuid::Uuid);

impl std::fmt::Display for PlayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_hyphenated())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AccountId(pub uuid::Uuid);

impl AccountId {
    pub fn to_string(&self) -> String {
        self.0.as_hyphenated().to_string()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MatchId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SeekId(u32);

impl SeekId {
    pub fn new(id: u32) -> Self {
        SeekId(id)
    }
}

impl std::fmt::Display for SeekId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GameId(u32);

impl GameId {
    pub fn new(id: u32) -> Self {
        GameId(id)
    }
}

impl std::fmt::Display for GameId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FinishedGameId(pub i64);

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GameType {
    Unrated,
    Rated,
    Tournament,
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
