use crate::domain::{PlayerId, RepoError, RepoRetrieveError};

#[async_trait::async_trait]
pub trait StatsRepository {
    async fn get_player_stats(&self, player_id: PlayerId)
    -> Result<PlayerStats, RepoRetrieveError>;
    async fn update_player_game(
        &self,
        player_id: PlayerId,
        result: GameOutcome,
        was_rated: bool,
    ) -> Result<(), RepoError>;
    async fn remove_player_stats(&self, player_id: PlayerId) -> Result<(), RepoError>;
}

#[derive(Clone, Debug)]
pub struct PlayerStats {
    pub rated_games_played: u32,
    pub games_played: u32,
    pub games_won: u32,
    pub games_lost: u32,
    pub games_drawn: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum GameOutcome {
    Win,
    Loss,
    Draw,
}
