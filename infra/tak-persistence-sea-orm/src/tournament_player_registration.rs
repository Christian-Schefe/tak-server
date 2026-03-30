use crate::create_db_pool;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait, QueryFilter};
use tak_persistence_sea_orm_entities::tournament_player_registration;
use tak_server_app::domain::TournamentId;
use tak_server_app::domain::tournament::{TournamentPlayer, TournamentPlayerRepository};
use tak_server_app::domain::{PlayerId, RepoError};

pub struct TournamentPlayerRegistrationRepositoryImpl {
    db: DatabaseConnection,
}

impl TournamentPlayerRegistrationRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
        Self { db }
    }
}

#[async_trait::async_trait]
impl TournamentPlayerRepository for TournamentPlayerRegistrationRepositoryImpl {
    async fn get_tournament_players(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Vec<TournamentPlayer>, RepoError> {
        let registration_models = tournament_player_registration::Entity::find()
            .filter(tournament_player_registration::Column::TournamentId.eq(tournament_id.0))
            .all(&self.db)
            .await
            .map_err(|e| {
                RepoError::StorageError(format!(
                    "Failed to retrieve player registrations for tournament {}: {}",
                    tournament_id.0, e
                ))
            })?;

        let player_ids = registration_models
            .into_iter()
            .map(|m| TournamentPlayer {
                player_id: PlayerId(m.player_id),
                score: m.score as u32,
            })
            .collect();

        Ok(player_ids)
    }

    async fn create_tournament_player(
        &self,
        tournament_id: TournamentId,
        player: TournamentPlayer,
    ) -> Result<(), RepoError> {
        let model = tournament_player_registration::ActiveModel {
            tournament_id: sea_orm::Set(tournament_id.0),
            player_id: sea_orm::Set(player.player_id.0),
            score: sea_orm::Set(player.score as i32),
        };
        match tournament_player_registration::Entity::insert(model)
            .on_conflict_do_nothing()
            .exec(&self.db)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(RepoError::StorageError(format!(
                "Failed to register player to tournament: {}",
                e
            ))),
        }
    }
    async fn remove_tournament_player(
        &self,
        tournament_id: TournamentId,
        player_id: PlayerId,
    ) -> Result<(), RepoError> {
        let model = tournament_player_registration::ActiveModel {
            tournament_id: sea_orm::Set(tournament_id.0),
            player_id: sea_orm::Set(player_id.0),
            ..Default::default()
        };
        match tournament_player_registration::Entity::delete(model)
            .exec(&self.db)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(RepoError::StorageError(format!(
                "Failed to unregister player from tournament: {}",
                e
            ))),
        }
    }
    async fn increase_player_score(
        &self,
        tournament_id: TournamentId,
        player_id: PlayerId,
        score_increase: u32,
    ) -> Result<(), RepoError> {
        tournament_player_registration::Entity::update_many()
            .col_expr(
                tournament_player_registration::Column::Score,
                sea_orm::sea_query::Expr::col(tournament_player_registration::Column::Score)
                    .add(score_increase as i32),
            )
            .filter(tournament_player_registration::Column::TournamentId.eq(tournament_id.0))
            .filter(tournament_player_registration::Column::PlayerId.eq(player_id.0))
            .exec(&self.db)
            .await
            .map_err(|e| {
                RepoError::StorageError(format!(
                    "Failed to increase player score in tournament: {}",
                    e
                ))
            })?;
        Ok(())
    }
}
