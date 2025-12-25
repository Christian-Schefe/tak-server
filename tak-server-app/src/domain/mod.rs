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

impl PlayerId {
    pub fn to_string(&self) -> String {
        self.0.as_hyphenated().to_string()
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GameId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FinishedGameId(pub i64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ListenerId(uuid::Uuid);

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
