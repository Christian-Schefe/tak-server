use std::collections::HashMap;
use std::sync::Arc;

use tak_domain::{
    MatchId, TournamentId, UserId,
    matches::{Match, MatchSettings, MatchStatus, MatchTournamentInfo},
    tournament::{
        Tournament, TournamentFormat, TournamentMetadata, TournamentPlayer, TournamentRound,
        TournamentStatus,
    },
};

use crate::{
    PortsCollection,
    ports::{
        matches::MatchRepository,
        moderation::ModerationRepository,
        rating::RatingRepository,
        tournament::{TournamentPlayerRepository, TournamentRepository, TournamentRoundRepository},
        transaction::{RepoError, TransactionManager},
    },
};

#[derive(Clone, Debug)]
pub struct TournamentMetadataView {
    pub tournament_id: TournamentId,
    pub name: String,
    pub tournament_format: TournamentFormat,
    pub match_settings: MatchSettings,
}

#[derive(Clone, Debug)]
pub struct TournamentView {
    pub metadata: TournamentMetadataView,
    pub status: TournamentStatus,
}

#[derive(Clone, Debug)]
pub struct TournamentDetailView {
    pub tournament: TournamentView,
    pub player_scores: HashMap<UserId, u32>,
    pub rounds: Vec<TournamentRoundView>,
}

#[derive(Clone, Debug)]
pub struct TournamentRoundView {
    pub matches: Vec<MatchId>,
    pub byes: Vec<UserId>,
}

impl TournamentMetadataView {
    pub fn from_metadata(tournament_id: TournamentId, metadata: TournamentMetadata) -> Self {
        Self {
            tournament_id,
            name: metadata.name,
            tournament_format: metadata.format,
            match_settings: metadata.match_settings,
        }
    }
}

impl TournamentView {
    pub fn from_tournament(tournament_id: TournamentId, tournament: Tournament) -> Self {
        Self {
            metadata: TournamentMetadataView::from_metadata(tournament_id, tournament.metadata),
            status: tournament.status,
        }
    }
}

impl TournamentDetailView {
    pub fn from_tournament(
        tournament_id: TournamentId,
        tournament: Tournament,
        tournament_players: Vec<TournamentPlayer>,
        rounds: Vec<TournamentRound>,
    ) -> Self {
        Self {
            tournament: TournamentView::from_tournament(tournament_id, tournament),
            player_scores: tournament_players
                .into_iter()
                .map(|tp| (tp.user_id, tp.score))
                .collect(),
            rounds: rounds
                .into_iter()
                .map(TournamentRoundView::from_tournament_round)
                .collect(),
        }
    }
}

impl TournamentRoundView {
    pub fn from_tournament_round(tournament_round: TournamentRound) -> Self {
        Self {
            matches: tournament_round.matches,
            byes: tournament_round.byes,
        }
    }
}

pub enum TournamentRegistrationError {
    TournamentNotFound,
    TournamentNotUpcoming,
    StorageError,
}

#[async_trait::async_trait]
pub trait TournamentUseCase {
    async fn get_tournaments(&self) -> Result<Vec<TournamentView>, ()>;
    async fn get_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Option<TournamentDetailView>, ()>;
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
    async fn register_player_in_tournament(
        &self,
        tournament_id: TournamentId,
        user_id: UserId,
    ) -> Result<(), TournamentRegistrationError>;
    async fn unregister_player_from_tournament(
        &self,
        tournament_id: TournamentId,
        user_id: UserId,
    ) -> Result<(), TournamentRegistrationError>;
}

pub struct TournamentUseCaseImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection> TournamentUseCaseImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }

    async fn is_admin(&self, ctx: &Ports::TrCtx, user_id: UserId) -> Result<bool, ()> {
        match self
            .ports
            .moderation_repository()
            .get_moderation(ctx, user_id)
            .await
        {
            Ok(Some(moderation)) => Ok(moderation.is_admin()),
            Ok(None) => Err(()),
            Err(RepoError(e)) => {
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
impl<Ports: PortsCollection + Send + Sync + 'static> TournamentUseCase
    for TournamentUseCaseImpl<Ports>
{
    async fn get_tournaments(&self) -> Result<Vec<TournamentView>, ()> {
        let ctx = self.ports.transaction_manager().simple();
        match self
            .ports
            .tournament_repository()
            .list_tournaments(&ctx)
            .await
        {
            Ok(tournaments) => Ok(tournaments
                .into_iter()
                .map(|(id, t)| TournamentView::from_tournament(id, t))
                .collect()),
            Err(RepoError(e)) => {
                tracing::error!("Failed to list tournaments: {:?}", e);
                Err(())
            }
        }
    }

    async fn get_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Option<TournamentDetailView>, ()> {
        let ctx = self.ports.transaction_manager().simple();
        let tournament_repo = self.ports.tournament_repository();
        let tournament_player_repo = self.ports.tournament_player_repository();
        let tournament_round_repo = self.ports.tournament_round_repository();
        match futures::try_join!(
            tournament_repo.get_tournament(&ctx, tournament_id),
            tournament_player_repo.get_tournament_players(&ctx, tournament_id),
            tournament_round_repo.get_tournament_rounds(&ctx, tournament_id),
        ) {
            Ok((tournament, tournament_players, rounds)) => {
                let Some(tournament) = tournament else {
                    return Ok(None);
                };
                Ok(Some(TournamentDetailView::from_tournament(
                    tournament_id,
                    tournament,
                    tournament_players,
                    rounds,
                )))
            }
            Err(RepoError(e)) => {
                tracing::error!("Failed to get tournament: {:?}", e);
                Err(())
            }
        }
    }

    #[tracing::instrument(skip(self))]
    async fn create_tournament(
        &self,
        user_id: UserId,
        name: String,
        format: TournamentFormat,
        match_settings: MatchSettings,
    ) -> Result<TournamentId, ()> {
        let ctx = self.ports.transaction_manager().simple();
        if !self.is_admin(&ctx, user_id).await? {
            return Err(());
        }
        let tournament = Tournament {
            metadata: TournamentMetadata {
                name,
                format,
                match_settings,
            },
            status: TournamentStatus::Upcoming {
                registration_open: false,
            },
        };
        match self
            .ports
            .tournament_repository()
            .create_tournament(&ctx, tournament)
            .await
        {
            Ok(tournament_id) => Ok(tournament_id),
            Err(RepoError(e)) => {
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
        let ctx = self.ports.transaction_manager().simple();
        if !self.is_admin(&ctx, user_id).await? {
            return Err(());
        }

        let tournament_player_repo = self.ports.tournament_player_repository();
        let tournament_repo = self.ports.tournament_repository();

        let tournament = self
            .ports
            .tournament_repository()
            .get_tournament(&ctx, tournament_id)
            .await
            .map_err(|RepoError(e)| {
                tracing::error!("Failed to retrieve tournament {}: {:?}", tournament_id, e);
            })?;

        let Some(tournament) = tournament else {
            tracing::warn!(
                "Attempted to begin tournament {} which does not exist",
                tournament_id
            );
            return Err(());
        };
        let TournamentStatus::Upcoming { .. } = tournament.status else {
            tracing::warn!(
                "Attempted to begin tournament {} which is not in Upcoming status",
                tournament_id
            );
            return Err(());
        };

        let tournament_players = tournament_player_repo
            .get_tournament_players(&ctx, tournament_id)
            .await
            .map_err(|RepoError(e)| {
                tracing::error!(
                    "Failed to retrieve tournament players for tournament {}: {:?}",
                    tournament_id,
                    e
                );
            })?;
        tournament_repo
            .set_tournament_status(&ctx, tournament_id, TournamentStatus::Ongoing)
            .await
            .map_err(|RepoError(e)| {
                tracing::error!(
                    "Failed to set tournament {} status to Ongoing: {:?}",
                    tournament_id,
                    e
                );
            })?;

        futures::future::try_join_all(tournament_players.into_iter().map(|player| {
            let tournament_player_repo = self.ports.tournament_player_repository();
            let rating_repo = self.ports.rating_repository();
            let ctx = &ctx;
            async move {
                let seeding_score =
                    match rating_repo.get_player_rating(&ctx, player.user_id).await? {
                        Some(rating) => rating.rating as i32,
                        // Player is a guest or not rated yet, use default rating of 1000
                        None => 1000,
                    };
                tournament_player_repo
                    .set_player_seeding_score(&ctx, tournament_id, player.user_id, seeding_score)
                    .await?;
                Ok(())
            }
        }))
        .await
        .map_err(|RepoError(e)| {
            tracing::error!(
                "Failed to set seeding scores for tournament {}: {:?}",
                tournament_id,
                e
            );
        })?;

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
        let ctx = self.ports.transaction_manager().simple();
        if !self.is_admin(&ctx, user_id).await? {
            return Err(());
        }
        let tournament = match self
            .ports
            .tournament_repository()
            .get_tournament(&ctx, tournament_id)
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
            Err(RepoError(e)) => {
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
            .match_repository()
            .get_matches_of_tournament(&ctx, tournament_id)
            .await
            .map_err(|RepoError(e)| {
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
            .tournament_player_repository()
            .get_tournament_players(&ctx, tournament_id)
            .await
            .map_err(|RepoError(e)| {
                tracing::error!(
                    "Failed to retrieve player registrations for tournament {}: {:?}",
                    tournament_id,
                    e
                );
            })?;

        all_players.sort_by_key(|x| (std::cmp::Reverse(x.seeding_score), x.user_id.0));

        let rounds = match self
            .ports
            .tournament_round_repository()
            .get_tournament_rounds(&ctx, tournament_id)
            .await
        {
            Ok(res) => res,
            Err(RepoError(e)) => {
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

        let Some(pairings) = tournament.metadata.format.generate_pairings(
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

        let bye_futures = pairings.byes.into_iter().map(|user_id| {
            let ctx = &ctx;
            async move {
                if let Err(RepoError(e)) = self
                    .ports
                    .tournament_player_repository()
                    .increase_player_score(&ctx, tournament_id, user_id, 2)
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
            }
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
                let ctx = &ctx;
                async move {
                    let match_id = match self
                        .ports
                        .match_repository()
                        .create_match(&ctx, match_data)
                        .await
                    {
                        Ok(id) => id,
                        Err(RepoError(e)) => {
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
        if let Err(RepoError(e)) = self
            .ports
            .tournament_round_repository()
            .create_tournament_round(&ctx, tournament_id, next_round_index, round)
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
        let ctx = self.ports.transaction_manager().simple();
        if !self.is_admin(&ctx, user_id).await? {
            return Err(());
        }
        let tournament = match self
            .ports
            .tournament_repository()
            .get_tournament(&ctx, tournament_id)
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
            Err(RepoError(e)) => {
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
            .match_repository()
            .get_matches_of_tournament(&ctx, tournament_id)
            .await
            .map_err(|RepoError(e)| {
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

        if let Err(RepoError(e)) = self
            .ports
            .tournament_repository()
            .set_tournament_status(&ctx, tournament_id, TournamentStatus::Completed)
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
        let ctx = self.ports.transaction_manager().simple();
        if !self.is_admin(&ctx, user_id).await? {
            return Err(());
        }
        let tournament = match self
            .ports
            .tournament_repository()
            .get_tournament(&ctx, tournament_id)
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
            Err(RepoError(e)) => {
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
        if let Err(RepoError(e)) = self
            .ports
            .tournament_repository()
            .set_tournament_status(
                &ctx,
                tournament_id,
                TournamentStatus::Upcoming {
                    registration_open: open,
                },
            )
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
    #[tracing::instrument(skip(self))]
    async fn register_player_in_tournament(
        &self,
        tournament_id: TournamentId,
        user_id: UserId,
    ) -> Result<(), TournamentRegistrationError> {
        let ctx = self.ports.transaction_manager().simple();
        let tournament = match self
            .ports
            .tournament_repository()
            .get_tournament(&ctx, tournament_id)
            .await
        {
            Ok(Some(tournament)) => Ok(tournament),
            Ok(None) => Err(TournamentRegistrationError::TournamentNotFound),
            Err(RepoError(e)) => {
                tracing::error!(
                    "Failed to get tournament with id {}: {:?}",
                    tournament_id,
                    e
                );
                Err(TournamentRegistrationError::StorageError)
            }
        }?;
        let TournamentStatus::Upcoming {
            registration_open: true,
        } = tournament.status
        else {
            tracing::warn!(
                "Attempted to register player {} in tournament {} which is not upcoming or not open for registration",
                user_id,
                tournament_id
            );
            return Err(TournamentRegistrationError::TournamentNotUpcoming);
        };
        if let Err(RepoError(e)) = self
            .ports
            .tournament_player_repository()
            .create_tournament_player(&ctx, tournament_id, TournamentPlayer::new(user_id))
            .await
        {
            tracing::error!(
                "Failed to register player {} in tournament {}: {:?}",
                user_id,
                tournament_id,
                e
            );
            Err(TournamentRegistrationError::StorageError)
        } else {
            Ok(())
        }
    }

    #[tracing::instrument(skip(self))]
    async fn unregister_player_from_tournament(
        &self,
        tournament_id: TournamentId,
        user_id: UserId,
    ) -> Result<(), TournamentRegistrationError> {
        let ctx = self.ports.transaction_manager().simple();
        let tournament = match self
            .ports
            .tournament_repository()
            .get_tournament(&ctx, tournament_id)
            .await
        {
            Ok(Some(tournament)) => Ok(tournament),
            Ok(None) => {
                return Err(TournamentRegistrationError::TournamentNotFound);
            }
            Err(RepoError(e)) => {
                tracing::error!(
                    "Failed to get tournament with id {}: {:?}",
                    tournament_id,
                    e
                );
                Err(TournamentRegistrationError::StorageError)
            }
        }?;
        let TournamentStatus::Upcoming {
            registration_open: true,
        } = tournament.status
        else {
            tracing::warn!(
                "Attempted to unregister player {} in tournament {} which is not upcoming or not open for registration",
                user_id,
                tournament_id
            );
            return Err(TournamentRegistrationError::TournamentNotUpcoming);
        };
        if let Err(RepoError(e)) = self
            .ports
            .tournament_player_repository()
            .remove_tournament_player(&ctx, tournament_id, user_id)
            .await
        {
            tracing::error!(
                "Failed to unregister player {} from tournament {}: {:?}",
                user_id,
                tournament_id,
                e
            );
            Err(TournamentRegistrationError::StorageError)
        } else {
            Ok(())
        }
    }
}
