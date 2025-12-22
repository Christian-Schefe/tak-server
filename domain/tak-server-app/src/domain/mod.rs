pub mod account;
pub mod chat;
pub mod event;
pub mod game;
pub mod game_history;
pub mod player;
pub mod rating;
pub mod seek;
pub mod spectator;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayerId(uuid::Uuid);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SeekId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GameId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FinishedGameId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ListenerId(uuid::Uuid);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GameType {
    Unrated,
    Rated,
    Tournament,
}
