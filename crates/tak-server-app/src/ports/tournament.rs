use tak_domain::{
    TournamentId, UserId,
    tournament::{Tournament, TournamentPlayer, TournamentRound, TournamentStatus},
};

use crate::ports::transaction::{TransactionError, TransactionItem};

pub trait TournamentRepository<TrCtx> {
    fn create_tournament(
        &self,
        tournament: Tournament,
    ) -> TransactionItem<TrCtx, Result<TournamentId, TransactionError>>;
    fn list_tournaments(
        &self,
    ) -> TransactionItem<TrCtx, Result<Vec<(TournamentId, Tournament)>, TransactionError>>;
    fn get_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> TransactionItem<TrCtx, Result<Option<Tournament>, TransactionError>>;
    fn set_tournament_status(
        &self,
        tournament_id: TournamentId,
        status: TournamentStatus,
    ) -> TransactionItem<TrCtx, Result<(), TransactionError>>;
}

pub trait TournamentRoundRepository<TrCtx> {
    fn get_tournament_rounds(
        &self,
        tournament_id: TournamentId,
    ) -> TransactionItem<TrCtx, Result<Vec<TournamentRound>, TransactionError>>;
    fn create_tournament_round(
        &self,
        tournament_id: TournamentId,
        round_index: usize,
        tournament_round: TournamentRound,
    ) -> TransactionItem<TrCtx, Result<(), TransactionError>>;
}

pub trait TournamentPlayerRepository<TrCtx> {
    fn get_tournament_players(
        &self,
        tournament_id: TournamentId,
    ) -> TransactionItem<TrCtx, Result<Vec<TournamentPlayer>, TransactionError>>;
    fn create_tournament_player(
        &self,
        tournament_id: TournamentId,
        player: TournamentPlayer,
    ) -> TransactionItem<TrCtx, Result<(), TransactionError>>;
    fn increase_player_score(
        &self,
        tournament_id: TournamentId,
        user_id: UserId,
        score_increase: u32,
    ) -> TransactionItem<TrCtx, Result<(), TransactionError>>;
    fn remove_tournament_player(
        &self,
        tournament_id: TournamentId,
        user_id: UserId,
    ) -> TransactionItem<TrCtx, Result<(), TransactionError>>;
    fn set_player_seeding_score(
        &self,
        tournament_id: TournamentId,
        user_id: UserId,
        seeding_score: i32,
    ) -> TransactionItem<TrCtx, Result<(), TransactionError>>;
}
