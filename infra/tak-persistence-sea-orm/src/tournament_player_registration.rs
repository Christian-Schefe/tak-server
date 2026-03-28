use crate::create_db_pool;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use tak_persistence_sea_orm_entities::tournament_player_registration;
use tak_server_app::domain::TournamentId;
use tak_server_app::domain::tournament::TournamentPlayerRegistrationRepository;
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
impl TournamentPlayerRegistrationRepository for TournamentPlayerRegistrationRepositoryImpl {
    async fn get_registered_players(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Vec<PlayerId>, RepoError> {
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
            .map(|m| PlayerId(m.player_id))
            .collect();

        Ok(player_ids)
    }

    async fn register_player(
        &self,
        tournament_id: TournamentId,
        player_id: PlayerId,
    ) -> Result<(), RepoError> {
        let model = tournament_player_registration::ActiveModel {
            tournament_id: sea_orm::Set(tournament_id.0),
            player_id: sea_orm::Set(player_id.0),
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
    async fn unregister_player(
        &self,
        tournament_id: TournamentId,
        player_id: PlayerId,
    ) -> Result<(), RepoError> {
        let model = tournament_player_registration::ActiveModel {
            tournament_id: sea_orm::Set(tournament_id.0),
            player_id: sea_orm::Set(player_id.0),
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
}
