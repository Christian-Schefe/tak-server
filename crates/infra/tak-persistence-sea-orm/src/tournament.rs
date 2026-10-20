use crate::matches::JsonMatchSettings;
use crate::transaction::RepositoryContext;
use sea_orm::{ColumnTrait, EntityTrait, ExprTrait, QueryFilter};
use tak_domain::{
    MatchId, TournamentId, UserId,
    tournament::{
        Tournament, TournamentFormat, TournamentMetadata, TournamentPlayer, TournamentRound,
        TournamentStatus,
    },
};
use tak_persistence_sea_orm_entities::{tournament, tournament_players, tournament_round};
use tak_server_app::ports::tournament::{
    TournamentPlayerRepository, TournamentRepository, TournamentRoundRepository,
};
use tak_server_app::ports::transaction::RepoError;

pub struct TournamentRepositoryImpl;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum JsonTournamentFormat {
    Swiss { rounds: u32 },
    RoundRobin,
    GroupRoundRobin { group_size: u32 },
}

impl JsonTournamentFormat {
    fn to_tournament_type(&self) -> TournamentFormat {
        match self {
            JsonTournamentFormat::Swiss { rounds } => TournamentFormat::Swiss {
                rounds: *rounds as usize,
            },
            JsonTournamentFormat::RoundRobin => TournamentFormat::RoundRobin,
            JsonTournamentFormat::GroupRoundRobin { group_size } => {
                TournamentFormat::GroupRoundRobin {
                    group_size: *group_size as usize,
                }
            }
        }
    }
    fn from_tournament_type(tournament_type: &TournamentFormat) -> Self {
        match tournament_type {
            TournamentFormat::Swiss { rounds } => JsonTournamentFormat::Swiss {
                rounds: *rounds as u32,
            },
            TournamentFormat::GroupRoundRobin { group_size } => {
                JsonTournamentFormat::GroupRoundRobin {
                    group_size: *group_size as u32,
                }
            }
            TournamentFormat::RoundRobin => JsonTournamentFormat::RoundRobin,
        }
    }
}

impl TournamentRepositoryImpl {
    fn status_from_str(status: &str, registration_open: bool) -> Result<TournamentStatus, String> {
        match status {
            "upcoming" => Ok(TournamentStatus::Upcoming { registration_open }),
            "ongoing" => Ok(TournamentStatus::Ongoing),
            "completed" => Ok(TournamentStatus::Completed),
            other => Err(format!("Unknown tournament status: {}", other)),
        }
    }

    fn status_to_str(status: &TournamentStatus) -> (String, bool) {
        match status {
            TournamentStatus::Upcoming { registration_open } => {
                ("upcoming".to_string(), *registration_open)
            }
            TournamentStatus::Ongoing => ("ongoing".to_string(), false),
            TournamentStatus::Completed => ("completed".to_string(), false),
        }
    }

    fn tournament_from_model(model: tournament::Model) -> Result<Tournament, String> {
        let settings =
            serde_json::from_value::<JsonMatchSettings>(model.match_settings).map_err(|e| {
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
                match_settings: settings.to_match_settings(),
            },
            status: Self::status_from_str(&model.status, model.registration_open).map_err(|e| {
                format!(
                    "Failed to parse tournament status for tournament {}: {}",
                    model.tournament_id, e
                )
            })?,
        })
    }

    fn tournament_to_model(tournament: &Tournament) -> Result<tournament::ActiveModel, String> {
        let settings = JsonMatchSettings::from_match_settings(&tournament.metadata.match_settings);
        let (str_status, registration_open) = Self::status_to_str(&tournament.status);
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
            status: sea_orm::Set(str_status),
            registration_open: sea_orm::Set(registration_open),
            tournament_id: Default::default(),
        })
    }
}

#[async_trait::async_trait]
impl TournamentRepository<RepositoryContext> for TournamentRepositoryImpl {
    async fn create_tournament(
        &self,
        ctx: &RepositoryContext,
        tournament: Tournament,
    ) -> Result<TournamentId, RepoError> {
        let model = Self::tournament_to_model(&tournament)
            .map_err(|e| RepoError(format!("Failed to convert tournament to model: {}", e)))?;
        match tournament::Entity::insert(model).exec(ctx).await {
            Ok(res) => Ok(TournamentId(res.last_insert_id)),
            Err(e) => Err(RepoError(format!("Failed to create tournament: {}", e))),
        }
    }

    async fn list_tournaments(
        &self,
        ctx: &RepositoryContext,
    ) -> Result<Vec<(TournamentId, Tournament)>, RepoError> {
        let models = tournament::Entity::find()
            .all(ctx)
            .await
            .map_err(|e| RepoError(format!("Failed to list tournaments: {}", e)))?;

        let mut tournaments = Vec::new();
        for model in models {
            let tournament_id = TournamentId(model.tournament_id);
            let tournament = Self::tournament_from_model(model).map_err(|e| {
                RepoError(format!(
                    "Failed to convert tournament model to domain object for tournament {}: {}",
                    tournament_id.0, e
                ))
            })?;
            tournaments.push((tournament_id, tournament));
        }
        Ok(tournaments)
    }

    async fn get_tournament(
        &self,
        ctx: &RepositoryContext,
        tournament_id: TournamentId,
    ) -> Result<Option<Tournament>, RepoError> {
        let Some(model) = tournament::Entity::find_by_id(tournament_id.0)
            .one(ctx)
            .await
            .map_err(|e| {
                RepoError(format!(
                    "Failed to retrieve tournament {}: {}",
                    tournament_id.0, e
                ))
            })?
        else {
            return Ok(None);
        };

        let tournament = Self::tournament_from_model(model).map_err(|e| {
            RepoError(format!(
                "Failed to convert tournament model to domain object for tournament {}: {}",
                tournament_id.0, e
            ))
        })?;

        Ok(Some(tournament))
    }

    async fn set_tournament_status(
        &self,
        ctx: &RepositoryContext,
        tournament_id: TournamentId,
        status: TournamentStatus,
    ) -> Result<(), RepoError> {
        let (str_status, registration_open) = Self::status_to_str(&status);
        let model = tournament::ActiveModel {
            tournament_id: sea_orm::Set(tournament_id.0),
            status: sea_orm::Set(str_status),
            registration_open: sea_orm::Set(registration_open),
            ..Default::default()
        };
        tournament::Entity::update(model)
            .exec(ctx)
            .await
            .map_err(|e| {
                RepoError(format!(
                    "Failed to update tournament status for tournament {}: {}",
                    tournament_id.0, e
                ))
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
    ) -> Result<Vec<TournamentPlayer>, RepoError> {
        let registration_models = tournament_players::Entity::find()
            .filter(tournament_players::Column::TournamentId.eq(tournament_id.0))
            .all(ctx)
            .await
            .map_err(|e| {
                RepoError(format!(
                    "Failed to retrieve player registrations for tournament {}: {}",
                    tournament_id.0, e
                ))
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
    ) -> Result<(), RepoError> {
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
            Err(e) => Err(RepoError(format!(
                "Failed to register player to tournament: {}",
                e
            ))),
        }
    }

    async fn remove_tournament_player(
        &self,
        ctx: &RepositoryContext,
        tournament_id: TournamentId,
        user_id: UserId,
    ) -> Result<(), RepoError> {
        let model = tournament_players::ActiveModel {
            tournament_id: sea_orm::Set(tournament_id.0),
            user_id: sea_orm::Set(user_id.0),
            ..Default::default()
        };
        match tournament_players::Entity::delete(model).exec(ctx).await {
            Ok(_) => Ok(()),
            Err(e) => Err(RepoError(format!(
                "Failed to unregister player from tournament: {}",
                e
            ))),
        }
    }

    async fn increase_player_score(
        &self,
        ctx: &RepositoryContext,
        tournament_id: TournamentId,
        user_id: UserId,
        score_increase: u32,
    ) -> Result<(), RepoError> {
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
            Err(e) => Err(RepoError(format!(
                "Failed to increase player score in tournament: {}",
                e
            ))),
        }
    }

    async fn set_player_seeding_score(
        &self,
        ctx: &RepositoryContext,
        tournament_id: TournamentId,
        user_id: UserId,
        seeding_score: i32,
    ) -> Result<(), RepoError> {
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
            Err(e) => Err(RepoError(format!(
                "Failed to set player seeding score in tournament: {}",
                e
            ))),
        }
    }
}

pub struct TournamentRoundRepositoryImpl;

impl TournamentRoundRepositoryImpl {
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
                    UserId::try_from(s).map_err(|e| {
                        format!(
                            "Failed to parse user ID from bye in tournament {} round {}: {}",
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
impl TournamentRoundRepository<RepositoryContext> for TournamentRoundRepositoryImpl {
    async fn get_tournament_rounds(
        &self,
        ctx: &RepositoryContext,
        tournament_id: TournamentId,
    ) -> Result<Vec<TournamentRound>, RepoError> {
        let round_models = tournament_round::Entity::find()
            .filter(tournament_round::Column::TournamentId.eq(tournament_id.0))
            .all(ctx)
            .await
            .map_err(|e| {
                RepoError(format!(
                    "Failed to retrieve tournament rounds for tournament {}: {}",
                    tournament_id.0, e
                ))
            })?;
        round_models
            .into_iter()
            .map(|model| {
                Self::round_from_model(model).map_err(|e| {
                    RepoError(format!(
                        "Failed to convert tournament round model to domain object: {}",
                        e
                    ))
                })
            })
            .collect()
    }
    async fn create_tournament_round(
        &self,
        ctx: &RepositoryContext,
        tournament_id: TournamentId,
        round_index: usize,
        tournament_round: TournamentRound,
    ) -> Result<(), RepoError> {
        let model =
            Self::round_to_model(tournament_id, round_index, &tournament_round).map_err(|e| {
                RepoError(format!(
                    "Failed to convert tournament round to model: {}",
                    e
                ))
            })?;
        match tournament_round::Entity::insert(model).exec(ctx).await {
            Ok(_) => Ok(()),
            Err(e) => Err(RepoError(format!(
                "Failed to create tournament round: {}",
                e
            ))),
        }
    }
}
