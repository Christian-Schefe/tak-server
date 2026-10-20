use std::sync::Arc;

use tak_domain::matches::Match;

use crate::{
    PortsCollection,
    ports::{tournament::TournamentPlayerRepository, transaction::RepoError},
};

#[async_trait::async_trait]
pub trait TournamentMatchWorkflow<RpCtx> {
    async fn handle_completed_match(
        &self,
        ctx: &RpCtx,
        match_entry: Match,
    ) -> Result<(), RepoError>;
}

pub struct TournamentMatchWorkflowImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection> TournamentMatchWorkflowImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static> TournamentMatchWorkflow<Ports::TrCtx>
    for TournamentMatchWorkflowImpl<Ports>
{
    #[tracing::instrument(skip(self, ctx))]
    async fn handle_completed_match(
        &self,
        ctx: &Ports::TrCtx,
        match_entry: Match,
    ) -> Result<(), RepoError> {
        let Some(tournament_info) = &match_entry.tournament_info else {
            return Ok(());
        };
        let winner = match_entry.get_winner();
        let repo = self.ports.tournament_player_repository();
        if let Some(winner_id) = winner {
            repo.increase_player_score(&ctx, tournament_info.tournament_id, winner_id, 2)
                .await
        } else {
            futures::try_join!(
                repo.increase_player_score(
                    &ctx,
                    tournament_info.tournament_id,
                    match_entry.player1.user_id,
                    1,
                ),
                repo.increase_player_score(
                    &ctx,
                    tournament_info.tournament_id,
                    match_entry.player2.user_id,
                    1,
                ),
            )?;
            Ok(())
        }
    }
}
