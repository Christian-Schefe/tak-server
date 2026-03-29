use std::sync::Arc;

use tak_core::TakGameSettings;

use crate::{
    domain::{
        TournamentId,
        matches::{Match, MatchMode, MatchRepository, MatchStatus, MatchTournamentInfo},
        tournament::{
            Tournament, TournamentMetadata, TournamentPlayerRegistrationRepository,
            TournamentRepository, TournamentStatus, TournamentType,
        },
    },
    workflow::matchmaking::create_game::CreateGameFromMatchWorkflow,
};

#[async_trait::async_trait]
pub trait HostTournamentUseCase {
    async fn create_tournament(
        &self,
        name: String,
        tournament_type: TournamentType,
        match_settings: TakGameSettings,
    ) -> Result<TournamentId, ()>;
    async fn begin_tournament(&self, tournament_id: TournamentId) -> Result<(), ()>;
    async fn start_next_round(&self, tournament_id: TournamentId) -> Result<(), ()>;
}

pub struct HostTournamentUseCaseImpl<
    TR: TournamentRepository,
    M: MatchRepository,
    TPR: TournamentPlayerRegistrationRepository,
    C: CreateGameFromMatchWorkflow,
> {
    tournament_repository: Arc<TR>,
    match_repository: Arc<M>,
    tournament_player_registration_repository: Arc<TPR>,
    create_game_workflow: Arc<C>,
}

impl<
    TR: TournamentRepository,
    M: MatchRepository,
    TPR: TournamentPlayerRegistrationRepository,
    C: CreateGameFromMatchWorkflow,
> HostTournamentUseCaseImpl<TR, M, TPR, C>
{
    pub fn new(
        tournament_repository: Arc<TR>,
        match_repository: Arc<M>,
        tournament_player_registration_repository: Arc<TPR>,
        create_game_workflow: Arc<C>,
    ) -> Self {
        Self {
            tournament_repository,
            match_repository,
            tournament_player_registration_repository,
            create_game_workflow,
        }
    }
}

#[async_trait::async_trait]
impl<
    TR: TournamentRepository + Send + Sync + 'static,
    M: MatchRepository + Send + Sync + 'static,
    TPR: TournamentPlayerRegistrationRepository + Send + Sync + 'static,
    C: CreateGameFromMatchWorkflow + Send + Sync + 'static,
> HostTournamentUseCase for HostTournamentUseCaseImpl<TR, M, TPR, C>
{
    #[tracing::instrument(skip(self))]
    async fn create_tournament(
        &self,
        name: String,
        tournament_type: TournamentType,
        match_settings: TakGameSettings,
    ) -> Result<TournamentId, ()> {
        let tournament = Tournament {
            metadata: TournamentMetadata {
                name,
                tournament_type,
                match_settings,
            },
            status: TournamentStatus::Upcoming,
        };
        match self
            .tournament_repository
            .create_tournament(tournament)
            .await
        {
            Ok(tournament_id) => Ok(tournament_id),
            Err(e) => {
                tracing::error!("Failed to create tournament: {:?}", e);
                Err(())
            }
        }
    }

    #[tracing::instrument(skip(self))]
    async fn begin_tournament(&self, tournament_id: TournamentId) -> Result<(), ()> {
        let tournament = match self
            .tournament_repository
            .get_tournament(tournament_id)
            .await
        {
            Ok(tournament) => tournament,
            Err(e) => {
                tracing::error!(
                    "Failed to begin tournament with id {}: {:?}",
                    tournament_id,
                    e
                );
                return Err(());
            }
        };
        let TournamentStatus::Upcoming = tournament.status else {
            tracing::warn!(
                "Attempted to begin tournament {} which is not in Upcoming status",
                tournament_id
            );
            return Err(());
        };
        if let Err(e) = self
            .tournament_repository
            .set_tournament_status(tournament_id, TournamentStatus::Ongoing)
            .await
        {
            tracing::error!(
                "Failed to set tournament {} status to Ongoing: {:?}",
                tournament_id,
                e
            );
            return Err(());
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn start_next_round(&self, tournament_id: TournamentId) -> Result<(), ()> {
        let tournament = match self
            .tournament_repository
            .get_tournament(tournament_id)
            .await
        {
            Ok(tournament) => tournament,
            Err(e) => {
                tracing::error!(
                    "Failed to get tournament with id {}: {:?}",
                    tournament_id,
                    e
                );
                return Err(());
            }
        };
        if let TournamentStatus::Completed = tournament.status {
            tracing::warn!(
                "Attempted to start next round of tournament {} which is already completed",
                tournament_id
            );
            return Err(());
        }
        let all_tournament_matches = match self
            .match_repository
            .get_matches_of_tournament(tournament_id)
            .await
        {
            Ok(matches) => matches,
            Err(e) => {
                tracing::error!(
                    "Failed to retrieve matches for tournament {}: {:?}",
                    tournament_id,
                    e
                );
                return Err(());
            }
        };
        if all_tournament_matches
            .iter()
            .any(|(_, match_entry)| !matches!(match_entry.status, MatchStatus::Completed))
        {
            tracing::warn!(
                "Attempted to start next round of tournament {} but not all matches from the current round are finished",
                tournament_id
            );
            return Err(());
        }
        let all_players = self
            .tournament_player_registration_repository
            .get_registered_players(tournament_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to retrieve player registrations for tournament {}: {:?}",
                    tournament_id,
                    e
                );
            })?;

        let round_index = all_tournament_matches
            .iter()
            .map(|(_, match_entry)| {
                match_entry
                    .tournament_info
                    .as_ref()
                    .map(|info| info.round)
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0)
            + 1;

        let pairings = tournament
            .metadata
            .tournament_type
            .generate_pairings(&all_players, round_index as usize);

        let create_match_futures = pairings.into_iter().enumerate().map(
            |(round_match_number, (player1, player2, color))| {
                let match_data = Match::new(
                    player1,
                    player2,
                    Some(color),
                    tournament.metadata.match_settings.clone(),
                    MatchMode::FixedGames(1),
                    true,
                    Some(MatchTournamentInfo {
                        tournament_id,
                        round: round_index,
                        round_match_number: round_match_number as u32,
                    }),
                );
                async move {
                    let match_id = match self.match_repository.create_match(match_data).await {
                        Ok(id) => id,
                        Err(e) => {
                            tracing::error!(
                                "Failed to create match for tournament {} round {}: {:?}",
                                tournament_id,
                                round_index,
                                e
                            );
                            return Err(());
                        }
                    };
                    self.create_game_workflow
                        .create_game_from_match(match_id)
                        .await
                        .map_err(move |e| {
                            tracing::error!(
                                "Failed to create game from match {}: {:?}",
                                match_id,
                                e
                            );
                        })
                }
            },
        );
        if let Err(_) = futures::future::join_all(create_match_futures)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
        {
            tracing::error!(
                "Failed to create all matches for tournament {} round {}",
                tournament_id,
                round_index
            );
            return Err(());
        }
        Ok(())
    }
}
