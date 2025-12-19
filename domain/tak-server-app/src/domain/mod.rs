pub mod game;
pub mod player;
pub mod seek;
pub mod spectator;

pub type PlayerId = uuid::Uuid;
pub type SeekId = u32;
pub type GameId = u32;
pub type ListenerId = uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GameType {
    Unrated,
    Rated,
    Tournament,
}
