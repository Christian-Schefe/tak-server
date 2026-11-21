use crate::{
    match_settings_from_json, match_settings_to_json, tournament_format_from_json,
    tournament_format_to_json, tournament_round_from_json, tournament_round_to_json,
    tournament_status_from_json, tournament_status_to_json, transaction::RepositoryContext,
};
use sea_orm::{
    ActiveValue::{NotSet, Set},
    ColumnTrait, EntityTrait, ExprTrait, QueryFilter,
};
use tak_domain::{
    TournamentId, UserId,
    tournament::{
        Tournament, TournamentMetadata, TournamentPlayer, TournamentRound, TournamentStatus,
    },
};
use tak_persistence_sea_orm_entities::{tournament, tournament_players, tournament_round};
use tak_server_app::{
    errors::{AppResult, InternalError, OneOf},
    ports::tournament::{
        TournamentPlayerRepository, TournamentRepository, TournamentRoundRepository,
    },
};

pub struct TournamentRepositoryImpl;

impl TournamentRepositoryImpl {
    fn tournament_from_model(model: tournament::Model) -> Tournament {
        Tournament {
            metadata: TournamentMetadata {
                name: model.name,
                format: tournament_format_from_json(&model.format),
                match_settings: match_settings_from_json(&model.match_settings),
            },
            status: tournament_status_from_json(&model.status),
        }
    }

    fn tournament_to_model(tournament: &Tournament) -> tournament::ActiveModel {
        tournament::ActiveModel {
            id: NotSet,
            name: Set(tournament.metadata.name.clone()),
            format: Set(tournament_format_to_json(&tournament.metadata.format)),
            match_settings: Set(match_settings_to_json(&tournament.metadata.match_settings)),
            status: Set(tournament_status_to_json(&tournament.status)),
        }
    }
}

#[async_trait::async_trait]
impl TournamentRepository<RepositoryContext> for TournamentRepositoryImpl {
    async fn create_tournament(
        &self,
        ctx: &RepositoryContext,
        tournament: Tournament,
    ) -> AppResult<TournamentId, (InternalError,)> {
        let model = Self::tournament_to_model(&tournament);
        match tournament::Entity::insert(model).exec(ctx).await {
            Ok(res) => Ok(TournamentId(res.last_insert_id)),
            Err(e) => {
                tracing::error!("Failed to create tournament: {}", e);
                Err(OneOf::new(InternalError))
            }
        }
    }

    async fn list_tournaments(
        &self,
        ctx: &RepositoryContext,
    ) -> AppResult<Vec<(TournamentId, Tournament)>, (InternalError,)> {
        let models = tournament::Entity::find().all(ctx).await.map_err(|e| {
            tracing::error!("Failed to retrieve tournaments: {}", e);
            OneOf::new(InternalError)
        })?;

        let mut tournaments = Vec::new();
        for model in models {
            let tournament_id = TournamentId(model.id);
            let tournament = Self::tournament_from_model(model);
            tournaments.push((tournament_id, tournament));
        }
        Ok(tournaments)
    }

    async fn get_tournament(
        &self,
        ctx: &RepositoryContext,
        tournament_id: TournamentId,
    ) -> AppResult<Option<Tournament>, (InternalError,)> {
        let Some(model) = tournament::Entity::find_by_id(tournament_id.0)
            .one(ctx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to retrieve tournament {}: {}", tournament_id.0, e);
                OneOf::new(InternalError)
            })?
        else {
            return Ok(None);
        };

        let tournament = Self::tournament_from_model(model);

        Ok(Some(tournament))
    }

    async fn set_tournament_status(
        &self,
        ctx: &RepositoryContext,
        tournament_id: TournamentId,
        status: TournamentStatus,
    ) -> AppResult<(), (InternalError,)> {
        let model = tournament::ActiveModel {
            id: Set(tournament_id.0),
            status: Set(tournament_status_to_json(&status)),
            ..Default::default()
        };
        tournament::Entity::update(model)
            .exec(ctx)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to update tournament status for tournament {}: {}",
                    tournament_id.0,
                    e
                );
                OneOf::new(InternalError)
            })?;
        Ok(())
    }
}

pub struct TournamentPlayerRegistrationRepositoryImpl;

#[async_trait::async_trait]
impl TournamentPlayerRepository<RepositoryContext> for TournamentPlayerRegistrationRepositoryImpl {
    async fn get_tournament_players(
        &self,
        ctx: &RepositoryContext,
        tournament_id: TournamentId,
    ) -> AppResult<Vec<TournamentPlayer>, (InternalError,)> {
        let registration_models = tournament_players::Entity::find()
            .filter(tournament_players::Column::TournamentId.eq(tournament_id.0))
            .all(ctx)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to retrieve player registrations for tournament {}: {}",
                    tournament_id.0,
                    e
                );
                OneOf::new(InternalError)
            })?;

        let user_ids = registration_models
            .into_iter()
            .map(|m| TournamentPlayer {
                user_id: UserId(m.user_id),
                score: m.score as u32,
                seeding_score: m.seeding_score,
            })
            .collect();

        Ok(user_ids)
    }

    async fn create_tournament_player(
        &self,
        ctx: &RepositoryContext,
        tournament_id: TournamentId,
        player: TournamentPlayer,
    ) -> AppResult<(), (InternalError,)> {
        let model = tournament_players::ActiveModel {
            tournament_id: sea_orm::Set(tournament_id.0),
            user_id: sea_orm::Set(player.user_id.0),
            score: sea_orm::Set(player.score as i32),
            seeding_score: sea_orm::Set(player.seeding_score),
        };
        match tournament_players::Entity::insert(model)
            .on_conflict_do_nothing()
            .exec(ctx)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!(
                    "Failed to register player to tournament {}: {}",
                    tournament_id.0,
                    e
                );
                Err(OneOf::new(InternalError))
            }
        }
    }

    async fn remove_tournament_player(
        &self,
        ctx: &RepositoryContext,
        tournament_id: TournamentId,
        user_id: UserId,
    ) -> AppResult<(), (InternalError,)> {
        let model = tournament_players::ActiveModel {
            tournament_id: sea_orm::Set(tournament_id.0),
            user_id: sea_orm::Set(user_id.0),
            ..Default::default()
        };
        match tournament_players::Entity::delete(model).exec(ctx).await {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!(
                    "Failed to unregister player from tournament {}: {}",
                    tournament_id.0,
                    e
                );
                Err(OneOf::new(InternalError))
            }
        }
    }

    async fn increase_player_score(
        &self,
        ctx: &RepositoryContext,
        tournament_id: TournamentId,
        user_id: UserId,
        score_increase: u32,
    ) -> AppResult<(), (InternalError,)> {
        match tournament_players::Entity::update_many()
            .col_expr(
                tournament_players::Column::Score,
                sea_orm::sea_query::Expr::col(tournament_players::Column::Score)
                    .add(score_increase as i32),
            )
            .filter(tournament_players::Column::TournamentId.eq(tournament_id.0))
            .filter(tournament_players::Column::UserId.eq(user_id.0))
            .exec(ctx)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!(
                    "Failed to increase player score in tournament {}: {}",
                    tournament_id.0,
                    e
                );
                Err(OneOf::new(InternalError))
            }
        }
    }

    async fn set_player_seeding_score(
        &self,
        ctx: &RepositoryContext,
        tournament_id: TournamentId,
        user_id: UserId,
        seeding_score: i32,
    ) -> AppResult<(), (InternalError,)> {
        let active_model = tournament_players::ActiveModel {
            tournament_id: sea_orm::Set(tournament_id.0),
            user_id: sea_orm::Set(user_id.0),
            seeding_score: sea_orm::Set(seeding_score),
            ..Default::default()
        };
        match tournament_players::Entity::update(active_model)
            .exec(ctx)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!(
                    "Failed to set player seeding score in tournament {}: {}",
                    tournament_id.0,
                    e
                );
                Err(OneOf::new(InternalError))
            }
        }
    }
}

pub struct TournamentRoundRepositoryImpl;

impl TournamentRoundRepositoryImpl {
    fn round_to_model(
        tournament_id: TournamentId,
        round_index: usize,
        tournament_round: &TournamentRound,
    ) -> tournament_round::ActiveModel {
        let json_round = tournament_round_to_json(tournament_round);
        tournament_round::ActiveModel {
            tournament_id: sea_orm::Set(tournament_id.0),
            round_index: sea_orm::Set(round_index as i64),
            data: sea_orm::Set(json_round),
        }
    }

    fn round_from_model(model: tournament_round::Model) -> TournamentRound {
        tournament_round_from_json(&model.data)
    }
}

#[async_trait::async_trait]
impl TournamentRoundRepository<RepositoryContext> for TournamentRoundRepositoryImpl {
    async fn get_tournament_rounds(
        &self,
        ctx: &RepositoryContext,
        tournament_id: TournamentId,
    ) -> AppResult<Vec<TournamentRound>, (InternalError,)> {
        let round_models = tournament_round::Entity::find()
            .filter(tournament_round::Column::TournamentId.eq(tournament_id.0))
            .all(ctx)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to retrieve tournament rounds for tournament {}: {}",
                    tournament_id.0,
                    e
                );
                OneOf::new(InternalError)
            })?;
        Ok(round_models
            .into_iter()
            .map(|model| Self::round_from_model(model))
            .collect())
    }
    async fn create_tournament_round(
        &self,
        ctx: &RepositoryContext,
        tournament_id: TournamentId,
        round_index: usize,
        tournament_round: TournamentRound,
    ) -> AppResult<(), (InternalError,)> {
        let model = Self::round_to_model(tournament_id, round_index, &tournament_round);
        match tournament_round::Entity::insert(model).exec(ctx).await {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!(
                    "Failed to create tournament round for tournament {}: {}",
                    tournament_id.0,
                    e
                );
                Err(OneOf::new(InternalError))
            }
        }
    }
}
