use std::sync::Arc;

use crate::domain::{matches::Match, tournament::TournamentPlayerRepository};

#[async_trait::async_trait]
pub trait TournamentMatchWorkflow {
    async fn handle_completed_match(&self, match_entry: Match);
}

pub struct TournamentMatchWorkflowImpl<TPR: TournamentPlayerRepository> {
    tournament_player_repository: Arc<TPR>,
}

impl<TPR: TournamentPlayerRepository> TournamentMatchWorkflowImpl<TPR> {
    pub fn new(tournament_player_repository: Arc<TPR>) -> Self {
        Self {
            tournament_player_repository,
        }
    }
}

#[async_trait::async_trait]
impl<TPR: TournamentPlayerRepository + Send + Sync + 'static> TournamentMatchWorkflow
    for TournamentMatchWorkflowImpl<TPR>
{
    #[tracing::instrument(skip(self))]
    async fn handle_completed_match(&self, match_entry: Match) {
        let Some(tournament_info) = &match_entry.tournament_info else {
            return;
        };
        let winner = match_entry.get_winner();
        if let Some(winner_id) = winner {
            if let Err(e) = self
                .tournament_player_repository
                .increase_player_score(tournament_info.tournament_id, winner_id, 1)
                .await
            {
                tracing::error!(
                    "Failed to update tournament player score for player {:?} in tournament {:?}: {:?}",
                    winner_id,
                    tournament_info.tournament_id,
                    e
                );
            }
        }
    }
}
