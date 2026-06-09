use std::sync::Arc;

use crate::domain::{
    RepoError, TournamentId,
    matches::{Match, MatchRepository, MatchSettings, MatchStatus, MatchTournamentInfo},
    rating::RatingRepository,
    tournament::{
        Tournament, TournamentFormat, TournamentMetadata, TournamentPlayerRepository,
        TournamentRepository, TournamentRound, TournamentRoundRepository, TournamentStatus,
    },
};

#[async_trait::async_trait]
pub trait HostTournamentUseCase {
    async fn create_tournament(
        &self,
        name: String,
        tournament_format: TournamentFormat,
        match_settings: MatchSettings,
    ) -> Result<TournamentId, ()>;
    async fn begin_tournament(&self, tournament_id: TournamentId) -> Result<(), ()>;
    async fn start_next_round(&self, tournament_id: TournamentId) -> Result<(), ()>;
    async fn finish_tournament(&self, tournament_id: TournamentId) -> Result<(), ()>;
}

pub struct HostTournamentUseCaseImpl<
    TR: TournamentRepository,
    M: MatchRepository,
    TPR: TournamentPlayerRepository,
    TRR: TournamentRoundRepository,
    R: RatingRepository,
> {
    tournament_repository: Arc<TR>,
    match_repository: Arc<M>,
    tournament_player_repository: Arc<TPR>,
    tournament_round_repository: Arc<TRR>,
    rating_repository: Arc<R>,
}

impl<
    TR: TournamentRepository,
    M: MatchRepository,
    TPR: TournamentPlayerRepository,
    TRR: TournamentRoundRepository,
    R: RatingRepository,
> HostTournamentUseCaseImpl<TR, M, TPR, TRR, R>
{
    pub fn new(
        tournament_repository: Arc<TR>,
        match_repository: Arc<M>,
        tournament_player_repository: Arc<TPR>,
        tournament_round_repository: Arc<TRR>,
        rating_repository: Arc<R>,
    ) -> Self {
        Self {
            tournament_repository,
            match_repository,
            tournament_player_repository,
            tournament_round_repository,
            rating_repository,
        }
    }
}

#[async_trait::async_trait]
impl<
    TR: TournamentRepository + Send + Sync + 'static,
    M: MatchRepository + Send + Sync + 'static,
    TPR: TournamentPlayerRepository + Send + Sync + 'static,
    TRR: TournamentRoundRepository + Send + Sync + 'static,
    R: RatingRepository + Send + Sync + 'static,
> HostTournamentUseCase for HostTournamentUseCaseImpl<TR, M, TPR, TRR, R>
{
    #[tracing::instrument(skip(self))]
    async fn create_tournament(
        &self,
        name: String,
        tournament_format: TournamentFormat,
        match_settings: MatchSettings,
    ) -> Result<TournamentId, ()> {
        let tournament = Tournament {
            metadata: TournamentMetadata {
                name,
                tournament_format,
                match_settings,
            },
            status: TournamentStatus::Upcoming {
                registration_open: false,
            },
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
        let TournamentStatus::Upcoming { .. } = tournament.status else {
            tracing::warn!(
                "Attempted to begin tournament {} which is not in Upcoming status",
                tournament_id
            );
            return Err(());
        };

        let tournament_players = match self
            .tournament_player_repository
            .get_tournament_players(tournament_id)
            .await
        {
            Ok(players) => players,
            Err(e) => {
                tracing::error!(
                    "Failed to retrieve player registrations for tournament {}: {:?}",
                    tournament_id,
                    e
                );
                return Err(());
            }
        };

        let set_seeding_futures = tournament_players.into_iter().map(|player| async move {
            let rating = self
                .rating_repository
                .get_player_rating(player.player_id)
                .await
                .map_err(|e| {
                    tracing::error!(
                        "Failed to retrieve rating for player {}: {:?}",
                        player.player_id,
                        e
                    );
                })?;
            if let Err(e) = self
                .tournament_player_repository
                .set_player_seeding_score(tournament_id, player.player_id, rating.rating as i32)
                .await
            {
                tracing::error!(
                    %tournament_id,
                    %player.player_id,
                    "Failed to set tournament player seeding score: {:?}",
                    e
                );
                return Err(());
            }
            Ok(())
        });

        if let Err(_) = futures::future::join_all(set_seeding_futures)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
        {
            tracing::error!(
                "Failed to set seeding scores for players in tournament {}",
                tournament_id
            );
            return Err(());
        }

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

        if let Err(()) = self.start_next_round(tournament_id).await {
            tracing::error!(
                "Failed to start first round of tournament {} after beginning it",
                tournament_id
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
        let TournamentStatus::Ongoing = tournament.status else {
            tracing::warn!(
                "Attempted to start next round of tournament {} which is not in Ongoing status",
                tournament_id
            );
            return Err(());
        };
        let tournament_matches = self
            .match_repository
            .get_matches_of_tournament(tournament_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to retrieve matches for tournament {}: {:?}",
                    tournament_id,
                    e
                );
            })?;
        if tournament_matches
            .iter()
            .any(|(_, match_entry)| !matches!(match_entry.status, MatchStatus::Completed))
        {
            tracing::warn!(
                "Attempted to start next round of tournament {} but not all matches from the current round are finished",
                tournament_id
            );
            return Err(());
        }
        let mut all_players = self
            .tournament_player_repository
            .get_tournament_players(tournament_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to retrieve player registrations for tournament {}: {:?}",
                    tournament_id,
                    e
                );
            })?;

        all_players.sort_by_key(|x| (std::cmp::Reverse(x.seeding_score), x.player_id.0));

        let rounds = match self
            .tournament_round_repository
            .get_tournament_rounds(tournament_id)
            .await
        {
            Ok(res) => res,
            Err(RepoError::StorageError(e)) => {
                tracing::error!(
                    "Failed to retrieve matches for tournament {}: {:?}",
                    tournament_id,
                    e
                );
                return Err(());
            }
        };
        let next_round_index = rounds.len();

        let previous_matches = tournament_matches
            .into_iter()
            .map(|(_, match_entry)| match_entry)
            .collect::<Vec<_>>();

        let Some(pairings) = tournament.metadata.tournament_format.generate_pairings(
            &all_players,
            &previous_matches,
            next_round_index,
        ) else {
            tracing::error!(
                "Attempted to start next round of tournament {} but tournament format indicates the tournament is already finished",
                tournament_id
            );
            return Err(());
        };

        let bye_futures = pairings.byes.into_iter().map(|player_id| async move {
            if let Err(e) = self
                .tournament_player_repository
                .increase_player_score(tournament_id, player_id, 2)
                .await
            {
                tracing::error!(
                    %player_id,
                    %tournament_id,
                    "Failed to update tournament player score: {:?}",
                    e
                );
                return Err(());
            }
            Ok(player_id)
        });

        let pairing_futures = pairings.pairings.into_iter().enumerate().map(
            |(round_match_number, (player1, player2, color))| {
                let match_data = Match::new(
                    player1,
                    player2,
                    Some(MatchTournamentInfo {
                        tournament_id,
                        round: next_round_index as u32,
                        round_match_number: round_match_number as u32,
                    }),
                    tournament.metadata.match_settings.clone(),
                    color,
                );
                async move {
                    let match_id = match self.match_repository.create_match(match_data).await {
                        Ok(id) => id,
                        Err(e) => {
                            tracing::error!(
                                "Failed to create match for tournament {} round {}: {:?}",
                                tournament_id,
                                next_round_index,
                                e
                            );
                            return Err(());
                        }
                    };
                    tracing::info!(
                        "Created match {} for tournament {} round {}",
                        match_id,
                        tournament_id,
                        next_round_index
                    );
                    Ok(match_id)
                }
            },
        );
        let (bye_results, pairing_results) = futures::join!(
            futures::future::join_all(bye_futures),
            futures::future::join_all(pairing_futures)
        );
        let byes = match bye_results.into_iter().collect::<Result<Vec<_>, _>>() {
            Ok(bye_player_ids) => bye_player_ids,
            Err(_) => {
                tracing::error!(
                    "Failed to process byes for tournament {} round {}",
                    tournament_id,
                    next_round_index
                );
                return Err(());
            }
        };
        let pairing_match_ids = match pairing_results.into_iter().collect::<Result<Vec<_>, _>>() {
            Ok(match_ids) => match_ids,
            Err(_) => {
                tracing::error!(
                    "Failed to create matches for tournament {} round {}",
                    tournament_id,
                    next_round_index
                );
                return Err(());
            }
        };
        let round = TournamentRound::new(pairing_match_ids, byes);
        if let Err(e) = self
            .tournament_round_repository
            .create_tournament_round(tournament_id, next_round_index, round)
            .await
        {
            tracing::error!(
                "Failed to create tournament round for tournament {} round {}: {:?}",
                tournament_id,
                next_round_index,
                e
            );
            return Err(());
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn finish_tournament(&self, tournament_id: TournamentId) -> Result<(), ()> {
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
        let TournamentStatus::Ongoing = tournament.status else {
            tracing::warn!(
                "Attempted to finish tournament {} which is not in Ongoing status",
                tournament_id
            );
            return Err(());
        };
        let tournament_matches = self
            .match_repository
            .get_matches_of_tournament(tournament_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to retrieve matches for tournament {}: {:?}",
                    tournament_id,
                    e
                );
            })?;

        if tournament_matches
            .iter()
            .any(|(_, match_entry)| !matches!(match_entry.status, MatchStatus::Completed))
        {
            tracing::warn!(
                "Attempted to finish tournament {} but not all matches are finished",
                tournament_id
            );
            return Err(());
        }

        if let Err(e) = self
            .tournament_repository
            .set_tournament_status(tournament_id, TournamentStatus::Completed)
            .await
        {
            tracing::error!(
                "Failed to set tournament {} status to Completed: {:?}",
                tournament_id,
                e
            );
            return Err(());
        }
        Ok(())
    }
}
