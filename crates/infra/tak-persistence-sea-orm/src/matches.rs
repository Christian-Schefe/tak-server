use crate::{JsonGameSettings, create_db_pool};
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use tak_core::TakPlayer;
use tak_persistence_sea_orm_entities::matches;
use tak_server_app::domain::matches::{
    Match, MatchMode, MatchPlayer, MatchRepository, MatchSettings, MatchStatus, MatchTournamentInfo,
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct JsonMatchSettings {
    game_settings: JsonGameSettings,
    match_mode: JsonMatchMode,
    is_rated: bool,
}

impl JsonMatchSettings {
    pub fn from_match_settings(settings: &MatchSettings) -> Self {
        JsonMatchSettings {
            game_settings: JsonGameSettings::from_game_settings(&settings.game_settings),
            match_mode: JsonMatchMode::from_match_mode(&settings.match_mode),
            is_rated: settings.is_rated,
        }
    }

    pub fn to_match_settings(&self) -> MatchSettings {
        MatchSettings {
            game_settings: self.game_settings.to_game_settings(),
            match_mode: self.match_mode.to_match_mode(),
            is_rated: self.is_rated,
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
                player1: MatchPlayer {
                    player_id: PlayerId(model.player1_id),
                    score: model.score_player1,
                },
                player2: MatchPlayer {
                    player_id: PlayerId(model.player2_id),
                    score: model.score_player2,
                },
                initial_color: match model.initial_color.as_str() {
                    "white" => Ok(TakPlayer::White),
                    "black" => Ok(TakPlayer::Black),
                    _ => Err(format!(
                        "Invalid initial color in database: {}",
                        model.initial_color
                    )),
                }?,
                settings: serde_json::from_value::<JsonMatchSettings>(model.settings)
                    .map_err(|e| {
                        format!("Failed to deserialize game settings from database: {}", e)
                    })?
                    .to_match_settings(),
                status: match model.status.as_str() {
                    "waiting" => Ok(MatchStatus::Waiting),
                    "ongoing" => Ok(MatchStatus::Ongoing),
                    "completed" => Ok(MatchStatus::Completed),
                    _ => Err(format!(
                        "Invalid match status in database: {}",
                        model.status
                    )),
                }?,
                games_played: model.games_played,
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
            player1_id: Set(match_entry.player1.player_id.0),
            player2_id: Set(match_entry.player2.player_id.0),
            initial_color: Set(match match_entry.initial_color {
                TakPlayer::White => "white".to_string(),
                TakPlayer::Black => "black".to_string(),
            }),
            settings: Set(serde_json::to_value(JsonMatchSettings::from_match_settings(
                &match_entry.settings,
            ))
            .map_err(|e| format!("Failed to serialize match settings for database: {}", e))?),
            status: Set(match match_entry.status {
                MatchStatus::Waiting => "waiting".to_string(),
                MatchStatus::Ongoing => "ongoing".to_string(),
                MatchStatus::Completed => "completed".to_string(),
            }),
            games_played: Set(match_entry.games_played),
            score_player1: Set(match_entry.player1.score),
            score_player2: Set(match_entry.player2.score),
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

    async fn get_matches_of_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Vec<(MatchId, Match)>, RepoError> {
        match matches::Entity::find()
            .filter(matches::Column::TournamentId.eq(tournament_id.0))
            .all(&self.db)
            .await
        {
            Ok(models) => models
                .into_iter()
                .map(Self::model_to_match)
                .collect::<Result<Vec<_>, String>>()
                .map_err(|e| {
                    RepoError::StorageError(format!("Failed to convert model to match: {}", e))
                }),
            Err(e) => Err(RepoError::StorageError(format!(
                "Failed to query matches from database: {}",
                e
            ))),
        }
    }
}
