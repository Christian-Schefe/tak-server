use crate::{JsonGameSettings, create_db_pool};
use sea_orm::{DatabaseConnection, EntityTrait};
use tak_persistence_sea_orm_entities::tournament;
use tak_server_app::domain::TournamentId;
use tak_server_app::domain::tournament::{
    Tournament, TournamentMetadata, TournamentRepository, TournamentStatus, TournamentType,
};
use tak_server_app::domain::{RepoError, RepoRetrieveError};

pub struct TournamentRepositoryImpl {
    db: DatabaseConnection,
}

impl TournamentRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
        Self { db }
    }

    fn status_from_str(status: &str) -> Result<TournamentStatus, String> {
        match status {
            "upcoming" => Ok(TournamentStatus::Upcoming),
            "ongoing" => Ok(TournamentStatus::Ongoing),
            "completed" => Ok(TournamentStatus::Completed),
            other => Err(format!("Unknown tournament status: {}", other)),
        }
    }

    fn status_to_str(status: &TournamentStatus) -> String {
        match status {
            TournamentStatus::Upcoming => "upcoming".to_string(),
            TournamentStatus::Ongoing => "ongoing".to_string(),
            TournamentStatus::Completed => "completed".to_string(),
        }
    }

    fn tournament_from_model(model: tournament::Model) -> Result<Tournament, String> {
        let settings =
            serde_json::from_value::<JsonGameSettings>(model.match_settings).map_err(|e| {
                format!(
                    "Failed to deserialize match settings for tournament {}: {}",
                    model.tournament_id, e
                )
            })?;
        Ok(Tournament {
            metadata: TournamentMetadata {
                name: model.name,
                tournament_type: match model.tournament_type.as_str() {
                    "swiss" => TournamentType::Swiss,
                    "round_robin" => TournamentType::RoundRobin,
                    other => {
                        return Err(format!(
                            "Unknown tournament type '{}' for tournament {}",
                            other, model.tournament_id
                        ));
                    }
                },
                match_settings: settings.to_game_settings(),
            },
            status: Self::status_from_str(&model.status).map_err(|e| {
                format!(
                    "Failed to parse tournament status for tournament {}: {}",
                    model.tournament_id, e
                )
            })?,
        })
    }

    fn tournament_to_model(tournament: &Tournament) -> Result<tournament::ActiveModel, String> {
        let settings = JsonGameSettings::from_game_settings(&tournament.metadata.match_settings);
        Ok(tournament::ActiveModel {
            name: sea_orm::Set(tournament.metadata.name.clone()),
            tournament_type: sea_orm::Set(match tournament.metadata.tournament_type {
                TournamentType::Swiss => "swiss".to_string(),
                TournamentType::RoundRobin => "round_robin".to_string(),
            }),
            match_settings: sea_orm::Set(
                serde_json::to_value(&settings)
                    .map_err(|e| format!("Failed to serialize match settings: {}", e))?,
            ),
            status: sea_orm::Set(Self::status_to_str(&tournament.status)),
            tournament_id: Default::default(),
        })
    }
}

#[async_trait::async_trait]
impl TournamentRepository for TournamentRepositoryImpl {
    async fn create_tournament(&self, tournament: Tournament) -> Result<TournamentId, RepoError> {
        let model = Self::tournament_to_model(&tournament).map_err(|e| {
            RepoError::StorageError(format!("Failed to convert tournament to model: {}", e))
        })?;
        match tournament::Entity::insert(model).exec(&self.db).await {
            Ok(res) => Ok(TournamentId(res.last_insert_id)),
            Err(e) => Err(RepoError::StorageError(format!(
                "Failed to create tournament: {}",
                e
            ))),
        }
    }

    async fn list_tournaments(&self) -> Result<Vec<(TournamentId, Tournament)>, RepoError> {
        let models = tournament::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(format!("Failed to list tournaments: {}", e)))?;

        let mut tournaments = Vec::new();
        for model in models {
            let tournament_id = TournamentId(model.tournament_id);
            match Self::tournament_from_model(model) {
                Ok(tournament) => tournaments.push((tournament_id, tournament)),
                Err(e) => {
                    tracing::error!("Failed to convert tournament model to domain object: {}", e);
                    continue;
                }
            }
        }
        Ok(tournaments)
    }

    async fn get_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Tournament, RepoRetrieveError> {
        let model = tournament::Entity::find_by_id(tournament_id.0)
            .one(&self.db)
            .await
            .map_err(|e| {
                RepoRetrieveError::StorageError(format!(
                    "Failed to retrieve tournament {}: {}",
                    tournament_id.0, e
                ))
            })?
            .ok_or(RepoRetrieveError::NotFound)?;

        let tournament = Self::tournament_from_model(model).map_err(|e| {
            RepoRetrieveError::StorageError(format!(
                "Failed to convert tournament model to domain object for tournament {}: {}",
                tournament_id.0, e
            ))
        })?;

        Ok(tournament)
    }

    async fn set_tournament_status(
        &self,
        tournament_id: TournamentId,
        status: TournamentStatus,
    ) -> Result<(), RepoError> {
        let model = tournament::ActiveModel {
            tournament_id: sea_orm::Set(tournament_id.0),
            status: sea_orm::Set(Self::status_to_str(&status)),
            ..Default::default()
        };
        tournament::Entity::update(model)
            .exec(&self.db)
            .await
            .map_err(|e| {
                RepoError::StorageError(format!(
                    "Failed to update tournament status for tournament {}: {}",
                    tournament_id.0, e
                ))
            })?;
        Ok(())
    }
}
