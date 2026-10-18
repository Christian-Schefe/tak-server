use std::sync::Arc;

use tak_domain::matches::Match;

use crate::{
    PortsCollection,
    ports::{
        tournament::TournamentPlayerRepository,
        transaction::{TransactionError, TransactionItem},
    },
};

pub trait TournamentMatchWorkflow<TrCtx> {
    fn handle_completed_match(
        &self,
        match_entry: Match,
    ) -> Option<TransactionItem<TrCtx, Result<(), TransactionError>>>;
}

pub struct TournamentMatchWorkflowImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection> TournamentMatchWorkflowImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }
}

impl<Ports: PortsCollection + Send + Sync + 'static> TournamentMatchWorkflow<Ports::TrCtx>
    for TournamentMatchWorkflowImpl<Ports>
{
    #[tracing::instrument(skip(self))]
    fn handle_completed_match(
        &self,
        match_entry: Match,
    ) -> Option<TransactionItem<Ports::TrCtx, Result<(), TransactionError>>> {
        let Some(tournament_info) = &match_entry.tournament_info else {
            return None;
        };
        let winner = match_entry.get_winner();
        let repo = self.ports.tournament_player_repository();
        if let Some(winner_id) = winner {
            Some(repo.increase_player_score(tournament_info.tournament_id, winner_id, 2))
        } else {
            Some(
                TransactionItem::try_parallel(vec![
                    repo.increase_player_score(
                        tournament_info.tournament_id,
                        match_entry.player1.user_id,
                        1,
                    ),
                    repo.increase_player_score(
                        tournament_info.tournament_id,
                        match_entry.player2.user_id,
                        1,
                    ),
                ])
                .map_value(|_| ()),
            )
        }
    }
}
