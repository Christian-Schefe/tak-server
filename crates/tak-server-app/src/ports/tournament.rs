use tak_domain::{
    TournamentId, UserId,
    tournament::{Tournament, TournamentPlayer, TournamentRound, TournamentStatus},
};

use crate::errors::{AppResult, InternalError};

#[async_trait::async_trait]
pub trait TournamentRepository<TrCtx> {
    async fn create_tournament(
        &self,
        ctx: &TrCtx,
        tournament: Tournament,
    ) -> AppResult<TournamentId, (InternalError,)>;
    async fn list_tournaments(
        &self,
        ctx: &TrCtx,
    ) -> AppResult<Vec<(TournamentId, Tournament)>, (InternalError,)>;
    async fn get_tournament(
        &self,
        ctx: &TrCtx,
        tournament_id: TournamentId,
    ) -> AppResult<Option<Tournament>, (InternalError,)>;
    async fn set_tournament_status(
        &self,
        ctx: &TrCtx,
        tournament_id: TournamentId,
        status: TournamentStatus,
    ) -> AppResult<(), (InternalError,)>;
}

#[async_trait::async_trait]
pub trait TournamentRoundRepository<TrCtx> {
    async fn get_tournament_rounds(
        &self,
        ctx: &TrCtx,
        tournament_id: TournamentId,
    ) -> AppResult<Vec<TournamentRound>, (InternalError,)>;
    async fn create_tournament_round(
        &self,
        ctx: &TrCtx,
        tournament_id: TournamentId,
        round_index: usize,
        tournament_round: TournamentRound,
    ) -> AppResult<(), (InternalError,)>;
}

#[async_trait::async_trait]
pub trait TournamentPlayerRepository<TrCtx> {
    async fn get_tournament_players(
        &self,
        ctx: &TrCtx,
        tournament_id: TournamentId,
    ) -> AppResult<Vec<TournamentPlayer>, (InternalError,)>;
    async fn create_tournament_player(
        &self,
        ctx: &TrCtx,
        tournament_id: TournamentId,
        player: TournamentPlayer,
    ) -> AppResult<(), (InternalError,)>;
    async fn increase_player_score(
        &self,
        ctx: &TrCtx,
        tournament_id: TournamentId,
        user_id: UserId,
        score_increase: u32,
    ) -> AppResult<(), (InternalError,)>;
    async fn remove_tournament_player(
        &self,
        ctx: &TrCtx,
        tournament_id: TournamentId,
        user_id: UserId,
    ) -> AppResult<(), (InternalError,)>;
    async fn set_player_seeding_score(
        &self,
        ctx: &TrCtx,
        tournament_id: TournamentId,
        user_id: UserId,
        seeding_score: i32,
    ) -> AppResult<(), (InternalError,)>;
}
