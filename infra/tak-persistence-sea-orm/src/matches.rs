use crate::{JsonGameSettings, create_db_pool};
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use tak_core::TakPlayer;
use tak_persistence_sea_orm_entities::matches;
use tak_server_app::domain::matches::{
    Match, MatchMode, MatchRepository, MatchStatus, MatchTournamentInfo,
};
use tak_server_app::domain::{MatchId, PlayerId, RepoError, RepoRetrieveError, TournamentId};

pub struct MatchRepositoryImpl {
    db: DatabaseConnection,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum JsonMatchMode {
    Unlimited,
    FixedGames(u32),
    FirstTo(u32),
}

impl JsonMatchMode {
    fn from_match_mode(match_mode: &MatchMode) -> Self {
        match match_mode {
            MatchMode::Unlimited => JsonMatchMode::Unlimited,
            MatchMode::FixedGames(games) => JsonMatchMode::FixedGames(*games),
            MatchMode::FirstTo(games) => JsonMatchMode::FirstTo(*games),
        }
    }

    fn to_match_mode(&self) -> MatchMode {
        match self {
            JsonMatchMode::Unlimited => MatchMode::Unlimited,
            JsonMatchMode::FixedGames(games) => MatchMode::FixedGames(*games),
            JsonMatchMode::FirstTo(games) => MatchMode::FirstTo(*games),
        }
    }
}

impl MatchRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
        Self { db }
    }

    fn model_to_match(model: matches::Model) -> Result<(MatchId, Match), String> {
        Ok((
            MatchId(model.match_id),
            Match {
                player1: PlayerId(model.player1_id),
                player2: PlayerId(model.player2_id),
                initial_color: match model.initial_color.as_str() {
                    "white" => Ok(TakPlayer::White),
                    "black" => Ok(TakPlayer::Black),
                    _ => Err(format!(
                        "Invalid initial color in database: {}",
                        model.initial_color
                    )),
                }?,
                game_settings: serde_json::from_value::<JsonGameSettings>(model.game_settings)
                    .map_err(|e| {
                        format!("Failed to deserialize game settings from database: {}", e)
                    })?
                    .to_game_settings(),
                status: match model.status.as_str() {
                    "initial" => Ok(MatchStatus::Initial),
                    "waiting" => Ok(MatchStatus::Waiting),
                    "in_progress" => Ok(MatchStatus::InProgress),
                    "completed" => Ok(MatchStatus::Completed),
                    _ => Err(format!(
                        "Invalid match status in database: {}",
                        model.status
                    )),
                }?,
                match_mode: serde_json::from_value::<JsonMatchMode>(model.match_mode)
                    .map_err(|e| format!("Failed to deserialize match mode from database: {}", e))?
                    .to_match_mode(),
                games_played: model.games_played,
                half_score_player1: model.half_score_player1,
                half_score_player2: model.half_score_player2,
                is_rated: model.is_rated,
                tournament_info: if let Some(tournament_id) = model.tournament_id
                    && let Some(round) = model.tournament_round
                    && let Some(match_number) = model.tournament_round_match_number
                {
                    Some(MatchTournamentInfo {
                        tournament_id: TournamentId(tournament_id),
                        round: round as u32,
                        round_match_number: match_number as u32,
                    })
                } else {
                    None
                },
            },
        ))
    }

    fn match_to_model(
        match_id: Option<MatchId>,
        match_entry: &Match,
    ) -> Result<matches::ActiveModel, String> {
        Ok(matches::ActiveModel {
            match_id: match_id.map(|id| Set(id.0)).unwrap_or_default(),
            player1_id: Set(match_entry.player1.0),
            player2_id: Set(match_entry.player2.0),
            initial_color: Set(match match_entry.initial_color {
                TakPlayer::White => "white".to_string(),
                TakPlayer::Black => "black".to_string(),
            }),
            game_settings: Set(serde_json::to_value(JsonGameSettings::from_game_settings(
                &match_entry.game_settings,
            ))
            .map_err(|e| format!("Failed to serialize game settings for database: {}", e))?),
            status: Set(match match_entry.status {
                MatchStatus::Initial => "initial".to_string(),
                MatchStatus::Waiting => "waiting".to_string(),
                MatchStatus::InProgress => "in_progress".to_string(),
                MatchStatus::Completed => "completed".to_string(),
            }),
            match_mode: Set(serde_json::to_value(JsonMatchMode::from_match_mode(
                &match_entry.match_mode,
            ))
            .map_err(|e| format!("Failed to serialize match mode for database: {}", e))?),
            games_played: Set(match_entry.games_played),
            half_score_player1: Set(match_entry.half_score_player1),
            half_score_player2: Set(match_entry.half_score_player2),
            is_rated: Set(match_entry.is_rated),
            tournament_id: Set(match_entry
                .tournament_info
                .as_ref()
                .map(|info| info.tournament_id.0)),
            tournament_round: Set(match_entry
                .tournament_info
                .as_ref()
                .map(|info| info.round as i64)),
            tournament_round_match_number: Set(match_entry
                .tournament_info
                .as_ref()
                .map(|info| info.round_match_number as i64)),
        })
    }
}

#[async_trait::async_trait]
impl MatchRepository for MatchRepositoryImpl {
    async fn create_match(&self, new_match: Match) -> Result<MatchId, RepoError> {
        let active_model = Self::match_to_model(None, &new_match).map_err(|e| {
            RepoError::StorageError(format!("Failed to convert match to model: {}", e))
        })?;
        match matches::Entity::insert(active_model).exec(&self.db).await {
            Ok(inserted) => Ok(MatchId(inserted.last_insert_id)),
            Err(e) => Err(RepoError::StorageError(format!(
                "Failed to insert match into database: {}",
                e
            ))),
        }
    }

    async fn get_match(&self, match_id: MatchId) -> Result<Match, RepoRetrieveError> {
        match matches::Entity::find_by_id(match_id.0).one(&self.db).await {
            Ok(Some(model)) => Self::model_to_match(model)
                .map_err(|e| {
                    RepoRetrieveError::StorageError(format!(
                        "Failed to convert model to match: {}",
                        e
                    ))
                })
                .map(|(_, match_entry)| match_entry),
            Ok(None) => Err(RepoRetrieveError::NotFound),
            Err(e) => Err(RepoRetrieveError::StorageError(format!(
                "Failed to query match from database: {}",
                e
            ))),
        }
    }

    async fn get_matches_of_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Vec<(MatchId, Match)>, RepoError> {
        match matches::Entity::find()
            .filter(matches::Column::TournamentId.eq(tournament_id.0))
            .all(&self.db)
            .await
        {
            Ok(models) => {
                let mut matches = Vec::new();
                for model in models {
                    match Self::model_to_match(model) {
                        Ok(match_data) => matches.push(match_data),
                        Err(e) => {
                            tracing::error!(
                                "Failed to convert match model to domain object for tournament {}: {}",
                                tournament_id,
                                e
                            );
                            continue;
                        }
                    }
                }
                Ok(matches)
            }
            Err(e) => Err(RepoError::StorageError(format!(
                "Failed to query matches of tournament {} from database: {}",
                tournament_id, e
            ))),
        }
    }

    async fn update_match(&self, match_id: MatchId, updated_match: Match) -> Result<(), RepoError> {
        let active_model = Self::match_to_model(Some(match_id), &updated_match).map_err(|e| {
            RepoError::StorageError(format!("Failed to convert match to model: {}", e))
        })?;
        match matches::Entity::update(active_model).exec(&self.db).await {
            Ok(_) => Ok(()),
            Err(e) => Err(RepoError::StorageError(format!(
                "Failed to update match in database: {}",
                e
            ))),
        }
    }
}
