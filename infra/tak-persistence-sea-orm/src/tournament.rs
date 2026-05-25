use crate::{JsonGameSettings, create_db_pool};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait, QueryFilter};
use tak_persistence_sea_orm_entities::{
    tournament, tournament_player_registration, tournament_round,
};
use tak_server_app::domain::tournament::{
    Tournament, TournamentFormat, TournamentMetadata, TournamentPlayer, TournamentPlayerRepository,
    TournamentRepository, TournamentRound, TournamentRoundRepository, TournamentStatus,
};
use tak_server_app::domain::{MatchId, PlayerId, TournamentId};
use tak_server_app::domain::{RepoError, RepoRetrieveError};

pub struct TournamentRepositoryImpl {
    db: DatabaseConnection,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum JsonTournamentFormat {
    Swiss { rounds: u32 },
    RoundRobin,
}

impl JsonTournamentFormat {
    fn to_tournament_type(&self) -> TournamentFormat {
        match self {
            JsonTournamentFormat::Swiss { rounds } => TournamentFormat::Swiss {
                rounds: *rounds as usize,
            },
            JsonTournamentFormat::RoundRobin => TournamentFormat::RoundRobin,
        }
    }
    fn from_tournament_type(tournament_type: &TournamentFormat) -> Self {
        match tournament_type {
            TournamentFormat::Swiss { rounds } => JsonTournamentFormat::Swiss {
                rounds: *rounds as u32,
            },
            TournamentFormat::RoundRobin => JsonTournamentFormat::RoundRobin,
        }
    }
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
        let tournament_format_settings =
            serde_json::from_value::<JsonTournamentFormat>(model.tournament_format_settings)
                .map_err(|e| {
                    format!(
                        "Failed to deserialize tournament format settings for tournament {}: {}",
                        model.tournament_id, e
                    )
                })?;
        Ok(Tournament {
            metadata: TournamentMetadata {
                name: model.name,
                tournament_format: tournament_format_settings.to_tournament_type(),
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
            tournament_format_settings: sea_orm::Set(
                serde_json::to_value(&JsonTournamentFormat::from_tournament_type(
                    &tournament.metadata.tournament_format,
                ))
                .map_err(|e| format!("Failed to serialize tournament format settings: {}", e))?,
            ),
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

pub struct TournamentRoundRepositoryImpl {
    db: DatabaseConnection,
}

impl TournamentRoundRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
        Self { db }
    }

    fn round_to_model(
        tournament_id: TournamentId,
        round_index: usize,
        tournament_round: &TournamentRound,
    ) -> Result<tournament_round::ActiveModel, String> {
        let json_round = JsonTournamentRound {
            byes: tournament_round
                .byes
                .iter()
                .map(|p| p.to_string())
                .collect(),
            matches: tournament_round
                .matches
                .iter()
                .map(|m| m.to_string())
                .collect(),
        };
        Ok(tournament_round::ActiveModel {
            tournament_id: sea_orm::Set(tournament_id.0),
            round_index: sea_orm::Set(round_index as i64),
            data: sea_orm::Set(
                serde_json::to_value(&json_round)
                    .map_err(|e| format!("Failed to serialize tournament round: {}", e))?,
            ),
        })
    }

    fn round_from_model(model: tournament_round::Model) -> Result<TournamentRound, String> {
        let json_round =
            serde_json::from_value::<JsonTournamentRound>(model.data).map_err(|e| {
                format!(
                    "Failed to deserialize tournament round data for tournament {} round {}: {}",
                    model.tournament_id, model.round_index, e
                )
            })?;
        Ok(TournamentRound {
            byes: json_round
                .byes
                .into_iter()
                .map(|s| {
                    PlayerId::try_from(s).map_err(|e| {
                        format!(
                            "Failed to parse player ID from bye in tournament {} round {}: {}",
                            model.tournament_id, model.round_index, e
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
            matches: json_round
                .matches
                .into_iter()
                .map(|s| {
                    MatchId::try_from(s).map_err(|e| {
                        format!(
                            "Failed to parse match ID from tournament {} round {}: {}",
                            model.tournament_id, model.round_index, e
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonTournamentRound {
    byes: Vec<String>,
    matches: Vec<String>,
}

#[async_trait::async_trait]
impl TournamentRoundRepository for TournamentRoundRepositoryImpl {
    async fn get_tournament_rounds(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Vec<TournamentRound>, RepoError> {
        let round_models = tournament_round::Entity::find()
            .filter(tournament_round::Column::TournamentId.eq(tournament_id.0))
            .all(&self.db)
            .await
            .map_err(|e| {
                RepoError::StorageError(format!(
                    "Failed to retrieve tournament rounds for tournament {}: {}",
                    tournament_id.0, e
                ))
            })?;
        round_models
            .into_iter()
            .map(|model| {
                Self::round_from_model(model).map_err(|e| {
                    RepoError::StorageError(format!(
                        "Failed to convert tournament round model to domain object: {}",
                        e
                    ))
                })
            })
            .collect()
    }
    async fn create_tournament_round(
        &self,
        tournament_id: TournamentId,
        round_index: usize,
        tournament_round: TournamentRound,
    ) -> Result<(), RepoError> {
        let model =
            Self::round_to_model(tournament_id, round_index, &tournament_round).map_err(|e| {
                RepoError::StorageError(format!(
                    "Failed to convert tournament round to model: {}",
                    e
                ))
            })?;
        match tournament_round::Entity::insert(model).exec(&self.db).await {
            Ok(_) => Ok(()),
            Err(e) => Err(RepoError::StorageError(format!(
                "Failed to create tournament round: {}",
                e
            ))),
        }
    }
}
