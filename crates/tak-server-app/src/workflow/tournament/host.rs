use std::sync::Arc;

use tak_domain::{
    TournamentId, UserId,
    matches::{Match, MatchSettings, MatchStatus, MatchTournamentInfo},
    tournament::{
        Tournament, TournamentFormat, TournamentMetadata, TournamentRound, TournamentStatus,
    },
};

use crate::{
    PortsCollection,
    ports::{
        matches::MatchRepository,
        moderation::ModerationRepository,
        rating::RatingRepository,
        tournament::{TournamentPlayerRepository, TournamentRepository, TournamentRoundRepository},
        transaction::{TransactionError, TransactionItem, TransactionManager},
    },
};

#[async_trait::async_trait]
pub trait HostTournamentUseCase {
    async fn create_tournament(
        &self,
        user_id: UserId,
        name: String,
        tournament_format: TournamentFormat,
        match_settings: MatchSettings,
    ) -> Result<TournamentId, ()>;
    async fn begin_tournament(
        &self,
        user_id: UserId,
        tournament_id: TournamentId,
    ) -> Result<(), ()>;
    async fn start_next_round(
        &self,
        user_id: UserId,
        tournament_id: TournamentId,
    ) -> Result<(), ()>;
    async fn finish_tournament(
        &self,
        user_id: UserId,
        tournament_id: TournamentId,
    ) -> Result<(), ()>;
    async fn set_registration_open(
        &self,
        user_id: UserId,
        tournament_id: TournamentId,
        open: bool,
    ) -> Result<(), ()>;
}

pub struct HostTournamentUseCaseImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection> HostTournamentUseCaseImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }

    async fn is_admin(&self, user_id: UserId) -> Result<bool, ()> {
        match self
            .ports
            .transaction_manager()
            .execute(self.ports.moderation_repository().get_moderation(user_id))
            .await
        {
            Ok(Some(moderation)) => Ok(moderation.is_admin()),
            Ok(None) => Err(()),
            Err(TransactionError(e)) => {
                tracing::error!(
                    "Failed to retrieve moderation for user_id: {}: {:?}",
                    user_id,
                    e
                );
                Err(())
            }
        }
    }
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static> HostTournamentUseCase
    for HostTournamentUseCaseImpl<Ports>
{
    #[tracing::instrument(skip(self))]
    async fn create_tournament(
        &self,
        user_id: UserId,
        name: String,
        tournament_format: TournamentFormat,
        match_settings: MatchSettings,
    ) -> Result<TournamentId, ()> {
        if !self.is_admin(user_id).await? {
            return Err(());
        }
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
            .ports
            .transaction_manager()
            .execute(
                self.ports
                    .tournament_repository()
                    .create_tournament(tournament),
            )
            .await
        {
            Ok(tournament_id) => Ok(tournament_id),
            Err(TransactionError(e)) => {
                tracing::error!("Failed to create tournament: {:?}", e);
                Err(())
            }
        }
    }

    #[tracing::instrument(skip(self))]
    async fn begin_tournament(
        &self,
        user_id: UserId,
        tournament_id: TournamentId,
    ) -> Result<(), ()> {
        if !self.is_admin(user_id).await? {
            return Err(());
        }

        let rating_repo = self.ports.rating_repository();
        let tournament_player_repo = self.ports.tournament_player_repository();
        let tournament_repo = self.ports.tournament_repository();

        match self
            .ports
            .transaction_manager()
            .execute(
                self.ports
                    .tournament_repository()
                    .get_tournament(tournament_id)
                    .flat_map_value(move |tournament| {
                        let Some(tournament) = tournament else {
                            tracing::warn!(
                                "Attempted to begin tournament {} which does not exist",
                                tournament_id
                            );
                            return Ok(Err(()));
                        };
                        let TournamentStatus::Upcoming { .. } = tournament.status else {
                            tracing::warn!(
                                "Attempted to begin tournament {} which is not in Upcoming status",
                                tournament_id
                            );
                            return Ok(Err(()));
                        };
                        Ok(Ok(()))
                    })
                    .try_then(move |res| {
                        if let Err(e) = res {
                            return TransactionItem::constant(Ok(Err(e)));
                        }
                        let tournament_player_repo2 = tournament_player_repo.clone();
                        let rating_repo = rating_repo.clone();
                        tournament_player_repo
                            .get_tournament_players(tournament_id)
                            .try_simultaneous(
                                tournament_repo.set_tournament_status(
                                    tournament_id,
                                    TournamentStatus::Ongoing,
                                ),
                            )
                            .try_then(move |(tournament_players, _)| {
                                let mut items = Vec::new();
                                for player in tournament_players {
                                    let tournament_player_repo = tournament_player_repo2.clone();
                                    items.push(
                                        rating_repo
                                            .get_player_rating(player.user_id)
                                            .map_value(|rating| match rating {
                                                Some(rating) => rating.rating as i32,
                                                // Player is a guest or not rated yet, use default rating of 1000
                                                None => 1000,
                                            })
                                            .try_then(move |seeding_score| {
                                                tournament_player_repo.set_player_seeding_score(
                                                    tournament_id,
                                                    player.user_id,
                                                    seeding_score,
                                                )
                                            }),
                                    )
                                }
                                TransactionItem::try_parallel(items)
                            })
                            .map_value(|_| Ok(()))
                    }),
            )
            .await
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(())) => Err(()),
            Err(TransactionError(e)) => {
                tracing::error!("Failed to begin tournament {}: {:?}", tournament_id, e);
                Err(())
            }
        }?;

        if let Err(()) = self.start_next_round(user_id, tournament_id).await {
            tracing::error!(
                "Failed to start first round of tournament {} after beginning it",
                tournament_id
            );
            return Err(());
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn start_next_round(
        &self,
        user_id: UserId,
        tournament_id: TournamentId,
    ) -> Result<(), ()> {
        if !self.is_admin(user_id).await? {
            return Err(());
        }
        let tournament = match self
            .ports
            .transaction_manager()
            .execute(
                self.ports
                    .tournament_repository()
                    .get_tournament(tournament_id),
            )
            .await
        {
            Ok(Some(tournament)) => tournament,
            Ok(None) => {
                tracing::warn!(
                    "Attempted to start next round of tournament {} which does not exist",
                    tournament_id
                );
                return Err(());
            }
            Err(TransactionError(e)) => {
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
            .ports
            .transaction_manager()
            .execute(
                self.ports
                    .match_repository()
                    .get_matches_of_tournament(tournament_id),
            )
            .await
            .map_err(|TransactionError(e)| {
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
            .ports
            .transaction_manager()
            .execute(
                self.ports
                    .tournament_player_repository()
                    .get_tournament_players(tournament_id),
            )
            .await
            .map_err(|TransactionError(e)| {
                tracing::error!(
                    "Failed to retrieve player registrations for tournament {}: {:?}",
                    tournament_id,
                    e
                );
            })?;

        all_players.sort_by_key(|x| (std::cmp::Reverse(x.seeding_score), x.user_id.0));

        let rounds = match self
            .ports
            .transaction_manager()
            .execute(
                self.ports
                    .tournament_round_repository()
                    .get_tournament_rounds(tournament_id),
            )
            .await
        {
            Ok(res) => res,
            Err(TransactionError(e)) => {
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

        let bye_futures = pairings.byes.into_iter().map(|user_id| async move {
            if let Err(TransactionError(e)) = self
                .ports
                .transaction_manager()
                .execute(
                    self.ports
                        .tournament_player_repository()
                        .increase_player_score(tournament_id, user_id, 2),
                )
                .await
            {
                tracing::error!(
                    %user_id,
                    %tournament_id,
                    "Failed to update tournament player score: {:?}",
                    e
                );
                return Err(());
            }
            Ok(user_id)
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
                    let match_id = match self
                        .ports
                        .transaction_manager()
                        .execute(self.ports.match_repository().create_match(match_data))
                        .await
                    {
                        Ok(id) => id,
                        Err(TransactionError(e)) => {
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
            Ok(bye_ids) => bye_ids,
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
        if let Err(TransactionError(e)) = self
            .ports
            .transaction_manager()
            .execute(
                self.ports
                    .tournament_round_repository()
                    .create_tournament_round(tournament_id, next_round_index, round),
            )
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
    async fn finish_tournament(
        &self,
        user_id: UserId,
        tournament_id: TournamentId,
    ) -> Result<(), ()> {
        if !self.is_admin(user_id).await? {
            return Err(());
        }
        let tournament = match self
            .ports
            .transaction_manager()
            .execute(
                self.ports
                    .tournament_repository()
                    .get_tournament(tournament_id),
            )
            .await
        {
            Ok(Some(tournament)) => tournament,
            Ok(None) => {
                tracing::warn!(
                    "Attempted to finish tournament {} which does not exist",
                    tournament_id
                );
                return Err(());
            }
            Err(TransactionError(e)) => {
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
            .ports
            .transaction_manager()
            .execute(
                self.ports
                    .match_repository()
                    .get_matches_of_tournament(tournament_id),
            )
            .await
            .map_err(|TransactionError(e)| {
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

        if let Err(TransactionError(e)) = self
            .ports
            .transaction_manager()
            .execute(
                self.ports
                    .tournament_repository()
                    .set_tournament_status(tournament_id, TournamentStatus::Completed),
            )
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

    async fn set_registration_open(
        &self,
        user_id: UserId,
        tournament_id: TournamentId,
        open: bool,
    ) -> Result<(), ()> {
        if !self.is_admin(user_id).await? {
            return Err(());
        }
        let tournament = match self
            .ports
            .transaction_manager()
            .execute(
                self.ports
                    .tournament_repository()
                    .get_tournament(tournament_id),
            )
            .await
        {
            Ok(Some(tournament)) => tournament,
            Ok(None) => {
                tracing::warn!(
                    "Attempted to set registration open for tournament {} which does not exist",
                    tournament_id
                );
                return Err(());
            }
            Err(TransactionError(e)) => {
                tracing::error!(
                    "Failed to get tournament with id {}: {:?}",
                    tournament_id,
                    e
                );
                return Err(());
            }
        };
        let TournamentStatus::Upcoming { registration_open } = tournament.status else {
            tracing::warn!(
                "Attempted to set registration open for tournament {} which is not in Upcoming status",
                tournament_id
            );
            return Err(());
        };
        if registration_open == open {
            return Ok(());
        }
        if let Err(TransactionError(e)) = self
            .ports
            .transaction_manager()
            .execute(self.ports.tournament_repository().set_tournament_status(
                tournament_id,
                TournamentStatus::Upcoming {
                    registration_open: open,
                },
            ))
            .await
        {
            tracing::error!(
                "Failed to set registration open for tournament {}: {:?}",
                tournament_id,
                e
            );
            return Err(());
        }
        Ok(())
    }
}
