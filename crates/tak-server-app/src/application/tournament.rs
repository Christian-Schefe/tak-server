use std::collections::HashMap;
use std::sync::Arc;

use better_errors::OneOf;
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
    errors::{
        AppResult, InsufficientPermissions, InternalError, TournamentNotFound,
        UnexpectedTournamentStatus, UserNotFound,
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

#[async_trait::async_trait]
pub trait TournamentUseCase {
    async fn get_tournaments(&self) -> AppResult<Vec<TournamentView>, (InternalError,)>;
    async fn get_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> AppResult<Option<TournamentDetailView>, (InternalError,)>;
    async fn create_tournament(
        &self,
        user_id: UserId,
        name: String,
        tournament_format: TournamentFormat,
        match_settings: MatchSettings,
    ) -> AppResult<TournamentId, (InsufficientPermissions, UserNotFound, InternalError)>;
    async fn begin_tournament(
        &self,
        user_id: UserId,
        tournament_id: TournamentId,
    ) -> AppResult<
        (),
        (
            InsufficientPermissions,
            UserNotFound,
            TournamentNotFound,
            UnexpectedTournamentStatus,
            InternalError,
        ),
    >;
    async fn start_next_round(
        &self,
        user_id: UserId,
        tournament_id: TournamentId,
    ) -> AppResult<
        (),
        (
            TournamentNotFound,
            UnexpectedTournamentStatus,
            UserNotFound,
            InternalError,
        ),
    >;
    async fn finish_tournament(
        &self,
        user_id: UserId,
        tournament_id: TournamentId,
    ) -> AppResult<
        (),
        (
            InsufficientPermissions,
            UserNotFound,
            UnexpectedTournamentStatus,
            TournamentNotFound,
            InternalError,
        ),
    >;
    async fn set_registration_open(
        &self,
        user_id: UserId,
        tournament_id: TournamentId,
        open: bool,
    ) -> AppResult<
        (),
        (
            InsufficientPermissions,
            UserNotFound,
            TournamentNotFound,
            UnexpectedTournamentStatus,
            InternalError,
        ),
    >;
    async fn register_player_in_tournament(
        &self,
        tournament_id: TournamentId,
        user_id: UserId,
    ) -> AppResult<
        (),
        (
            TournamentNotFound,
            UnexpectedTournamentStatus,
            InternalError,
        ),
    >;
    async fn unregister_player_from_tournament(
        &self,
        tournament_id: TournamentId,
        user_id: UserId,
    ) -> AppResult<
        (),
        (
            TournamentNotFound,
            UnexpectedTournamentStatus,
            InternalError,
        ),
    >;
}

pub struct TournamentUseCaseImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection> TournamentUseCaseImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }

    async fn is_admin(
        &self,
        ctx: &Ports::RpCtx,
        user_id: UserId,
    ) -> AppResult<bool, (UserNotFound, InternalError)> {
        Ok(self
            .ports
            .moderation_repository()
            .get_moderation(ctx, user_id)
            .await
            .map_err(OneOf::broaden)?
            .ok_or(OneOf::new(UserNotFound))?
            .is_admin())
    }
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static> TournamentUseCase
    for TournamentUseCaseImpl<Ports>
{
    async fn get_tournaments(&self) -> AppResult<Vec<TournamentView>, (InternalError,)> {
        let ctx = self.ports.transaction_manager().simple();
        let tournaments = self
            .ports
            .tournament_repository()
            .list_tournaments(&ctx)
            .await
            .map_err(OneOf::broaden)?;
        Ok(tournaments
            .into_iter()
            .map(|(id, t)| TournamentView::from_tournament(id, t))
            .collect())
    }

    async fn get_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> AppResult<Option<TournamentDetailView>, (InternalError,)> {
        let ctx = self.ports.transaction_manager().simple();
        let tournament_repo = self.ports.tournament_repository();
        let tournament_player_repo = self.ports.tournament_player_repository();
        let tournament_round_repo = self.ports.tournament_round_repository();
        let (tournament, tournament_players, rounds) = futures::try_join!(
            tournament_repo.get_tournament(&ctx, tournament_id),
            tournament_player_repo.get_tournament_players(&ctx, tournament_id),
            tournament_round_repo.get_tournament_rounds(&ctx, tournament_id),
        )?;

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

    #[tracing::instrument(skip(self))]
    async fn create_tournament(
        &self,
        user_id: UserId,
        name: String,
        format: TournamentFormat,
        match_settings: MatchSettings,
    ) -> AppResult<TournamentId, (InsufficientPermissions, UserNotFound, InternalError)> {
        let ctx = self.ports.transaction_manager().simple();
        if !self.is_admin(&ctx, user_id).await.map_err(OneOf::broaden)? {
            return Err(OneOf::new(InsufficientPermissions));
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
        let tournament_id = self
            .ports
            .tournament_repository()
            .create_tournament(&ctx, tournament)
            .await
            .map_err(OneOf::broaden)?;
        Ok(tournament_id)
    }

    #[tracing::instrument(skip(self))]
    async fn begin_tournament(
        &self,
        user_id: UserId,
        tournament_id: TournamentId,
    ) -> AppResult<
        (),
        (
            InsufficientPermissions,
            UserNotFound,
            TournamentNotFound,
            UnexpectedTournamentStatus,
            InternalError,
        ),
    > {
        let ctx = self.ports.transaction_manager().simple();
        if !self.is_admin(&ctx, user_id).await.map_err(OneOf::broaden)? {
            return Err(OneOf::new(InsufficientPermissions));
        }

        let tournament_player_repo = self.ports.tournament_player_repository();
        let tournament_repo = self.ports.tournament_repository();

        let tournament = self
            .ports
            .tournament_repository()
            .get_tournament(&ctx, tournament_id)
            .await
            .map_err(OneOf::broaden)?
            .ok_or(OneOf::new(TournamentNotFound))?;

        let TournamentStatus::Upcoming { .. } = tournament.status else {
            return Err(OneOf::new(UnexpectedTournamentStatus));
        };

        let tournament_players = tournament_player_repo
            .get_tournament_players(&ctx, tournament_id)
            .await
            .map_err(OneOf::broaden)?;
        tournament_repo
            .set_tournament_status(&ctx, tournament_id, TournamentStatus::Ongoing)
            .await
            .map_err(OneOf::broaden)?;

        let res: Result<Vec<()>, OneOf<(InternalError,)>> =
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
                        .set_player_seeding_score(
                            &ctx,
                            tournament_id,
                            player.user_id,
                            seeding_score,
                        )
                        .await?;
                    Ok(())
                }
            }))
            .await;
        res.map_err(OneOf::broaden)?;

        self.start_next_round(user_id, tournament_id)
            .await
            .map_err(OneOf::broaden)?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn start_next_round(
        &self,
        user_id: UserId,
        tournament_id: TournamentId,
    ) -> AppResult<
        (),
        (
            TournamentNotFound,
            UnexpectedTournamentStatus,
            UserNotFound,
            InternalError,
        ),
    > {
        let ctx = self
            .ports
            .transaction_manager()
            .transaction()
            .await
            .map_err(OneOf::broaden)?;
        if !self.is_admin(&ctx, user_id).await.map_err(OneOf::broaden)? {
            return Err(OneOf::new(UserNotFound));
        }
        let tournament = self
            .ports
            .tournament_repository()
            .get_tournament(&ctx, tournament_id)
            .await
            .map_err(OneOf::broaden)?
            .ok_or(OneOf::new(TournamentNotFound))?;
        let TournamentStatus::Ongoing = tournament.status else {
            return Err(OneOf::new(UnexpectedTournamentStatus));
        };
        let tournament_matches = self
            .ports
            .match_repository()
            .get_matches_of_tournament(&ctx, tournament_id)
            .await
            .map_err(OneOf::broaden)?;
        if tournament_matches
            .iter()
            .any(|(_, match_entry)| !matches!(match_entry.status, MatchStatus::Completed))
        {
            tracing::warn!(
                "Attempted to start next round of tournament {} but not all matches from the current round are finished",
                tournament_id
            );
            return Err(OneOf::new(UnexpectedTournamentStatus));
        }
        let mut all_players = self
            .ports
            .tournament_player_repository()
            .get_tournament_players(&ctx, tournament_id)
            .await
            .map_err(OneOf::broaden)?;

        all_players.sort_by_key(|x| (std::cmp::Reverse(x.seeding_score), x.user_id.0));

        let rounds = self
            .ports
            .tournament_round_repository()
            .get_tournament_rounds(&ctx, tournament_id)
            .await
            .map_err(OneOf::broaden)?;
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
            return Err(OneOf::new(UnexpectedTournamentStatus));
        };

        let bye_futures = futures::future::try_join_all(pairings.byes.into_iter().map(|user_id| {
            let ctx = &ctx;
            async move {
                self.ports
                    .tournament_player_repository()
                    .increase_player_score(&ctx, tournament_id, user_id, 2)
                    .await
                    .map(|_| user_id)
            }
        }));

        let pairing_futures =
            futures::future::try_join_all(pairings.pairings.into_iter().enumerate().map(
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
                        let match_id = self
                            .ports
                            .match_repository()
                            .create_match(&ctx, match_data)
                            .await?;
                        tracing::info!(
                            "Created match {} for tournament {} round {}",
                            match_id,
                            tournament_id,
                            next_round_index
                        );
                        Ok(match_id)
                    }
                },
            ));
        let (byes, pairing_match_ids) =
            futures::try_join!(bye_futures, pairing_futures).map_err(OneOf::broaden)?;

        let round = TournamentRound::new(pairing_match_ids, byes);
        self.ports
            .tournament_round_repository()
            .create_tournament_round(&ctx, tournament_id, next_round_index, round)
            .await
            .map_err(OneOf::broaden)?;
        self.ports
            .transaction_manager()
            .commit(ctx)
            .await
            .map_err(OneOf::broaden)?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn finish_tournament(
        &self,
        user_id: UserId,
        tournament_id: TournamentId,
    ) -> AppResult<
        (),
        (
            InsufficientPermissions,
            UserNotFound,
            UnexpectedTournamentStatus,
            TournamentNotFound,
            InternalError,
        ),
    > {
        let ctx = self.ports.transaction_manager().simple();
        if !self.is_admin(&ctx, user_id).await.map_err(OneOf::broaden)? {
            return Err(OneOf::new(InsufficientPermissions));
        }
        let tournament = self
            .ports
            .tournament_repository()
            .get_tournament(&ctx, tournament_id)
            .await
            .map_err(OneOf::broaden)?
            .ok_or(OneOf::new(TournamentNotFound))?;
        let TournamentStatus::Ongoing = tournament.status else {
            tracing::warn!(
                "Attempted to finish tournament {} which is not in Ongoing status",
                tournament_id
            );
            return Err(OneOf::new(UnexpectedTournamentStatus));
        };
        let tournament_matches = self
            .ports
            .match_repository()
            .get_matches_of_tournament(&ctx, tournament_id)
            .await
            .map_err(OneOf::broaden)?;

        if tournament_matches
            .iter()
            .any(|(_, match_entry)| !matches!(match_entry.status, MatchStatus::Completed))
        {
            tracing::warn!(
                "Attempted to finish tournament {} but not all matches are finished",
                tournament_id
            );
            return Err(OneOf::new(UnexpectedTournamentStatus));
        }

        self.ports
            .tournament_repository()
            .set_tournament_status(&ctx, tournament_id, TournamentStatus::Completed)
            .await
            .map_err(OneOf::broaden)?;
        Ok(())
    }

    async fn set_registration_open(
        &self,
        user_id: UserId,
        tournament_id: TournamentId,
        open: bool,
    ) -> AppResult<
        (),
        (
            InsufficientPermissions,
            UserNotFound,
            TournamentNotFound,
            UnexpectedTournamentStatus,
            InternalError,
        ),
    > {
        let ctx = self.ports.transaction_manager().simple();
        if !self.is_admin(&ctx, user_id).await.map_err(OneOf::broaden)? {
            return Err(OneOf::new(InsufficientPermissions));
        }
        let tournament = self
            .ports
            .tournament_repository()
            .get_tournament(&ctx, tournament_id)
            .await
            .map_err(OneOf::broaden)?
            .ok_or(OneOf::new(TournamentNotFound))?;
        let TournamentStatus::Upcoming { registration_open } = tournament.status else {
            return Err(OneOf::new(UnexpectedTournamentStatus));
        };
        if registration_open == open {
            return Ok(());
        }
        self.ports
            .tournament_repository()
            .set_tournament_status(
                &ctx,
                tournament_id,
                TournamentStatus::Upcoming {
                    registration_open: open,
                },
            )
            .await
            .map_err(OneOf::broaden)?;
        Ok(())
    }
    #[tracing::instrument(skip(self))]
    async fn register_player_in_tournament(
        &self,
        tournament_id: TournamentId,
        user_id: UserId,
    ) -> AppResult<
        (),
        (
            TournamentNotFound,
            UnexpectedTournamentStatus,
            InternalError,
        ),
    > {
        let ctx = self.ports.transaction_manager().simple();
        let tournament = self
            .ports
            .tournament_repository()
            .get_tournament(&ctx, tournament_id)
            .await
            .map_err(OneOf::broaden)?
            .ok_or(OneOf::new(TournamentNotFound))?;
        let TournamentStatus::Upcoming {
            registration_open: true,
        } = tournament.status
        else {
            return Err(OneOf::new(UnexpectedTournamentStatus));
        };
        self.ports
            .tournament_player_repository()
            .create_tournament_player(&ctx, tournament_id, TournamentPlayer::new(user_id))
            .await
            .map_err(OneOf::broaden)?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn unregister_player_from_tournament(
        &self,
        tournament_id: TournamentId,
        user_id: UserId,
    ) -> AppResult<
        (),
        (
            TournamentNotFound,
            UnexpectedTournamentStatus,
            InternalError,
        ),
    > {
        let ctx = self.ports.transaction_manager().simple();
        let tournament = self
            .ports
            .tournament_repository()
            .get_tournament(&ctx, tournament_id)
            .await
            .map_err(OneOf::broaden)?
            .ok_or(OneOf::new(TournamentNotFound))?;
        let TournamentStatus::Upcoming {
            registration_open: true,
        } = tournament.status
        else {
            return Err(OneOf::new(UnexpectedTournamentStatus));
        };
        self.ports
            .tournament_player_repository()
            .remove_tournament_player(&ctx, tournament_id, user_id)
            .await
            .map_err(OneOf::broaden)?;
        Ok(())
    }
}
