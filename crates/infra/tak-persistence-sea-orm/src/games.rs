use std::time::Duration;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use serde::Deserialize;
use tak_core::{
    TakAction, TakBaseGameSettings, TakGameSettings, TakPlayer, TakReserve, TakTimeInfo,
    ptn::{action_from_ptn, action_to_ptn, game_result_from_string, game_result_to_string},
};
use tak_persistence_sea_orm_entities::game;
use tak_server_app::domain::{
    GameId, MatchId, PaginatedResponse, PlayerId, RepoError, RepoRetrieveError, SortOrder,
    game::{GameEvent, GameEventType, GameMetadata, GameOverEventType, request::GameRequest},
    game_history::{
        DateSelector, GameFinishedUpdate, GameIdSelector, GamePlayerFilter, GameQuery,
        GameRatingInfo, GameRecord, GameRepository, GameSortBy, PlayerSnapshot,
    },
};

use crate::{
    JsonTimeSettings, create_db_pool, db_error_to_repo_retrieve_error, tak_opening_from_string,
    tak_opening_to_string,
};

pub struct GameRepositoryImpl {
    db: DatabaseConnection,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonEventRecord {
    #[serde(with = "chrono::serde::ts_milliseconds")]
    timestamp: chrono::DateTime<chrono::Utc>,
    event: JsonEventRecordType,
    time_info: JsonTimeInfo,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum JsonEventRecordType {
    Action {
        #[serde(
            serialize_with = "serialize_action",
            deserialize_with = "deserialize_action"
        )]
        action: TakAction,
    },
    RequestSet {
        request_type: JsonRequest,
        request_player: JsonTakPlayer,
    },
    ActionUndone,
    TimeGiven {
        player: JsonTakPlayer,
        amount_ms: u64,
    },
    GameOver {
        game_over_type: JsonEventGameOverType,
    },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
enum JsonEventGameOverType {
    DrawAgreement,
    Action,
    Timeout,
    Resignation,
    Abandonment,
    Aborted,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonTimeInfo {
    white_remaining_ms: u64,
    black_remaining_ms: u64,
}

impl JsonTimeInfo {
    fn from_time_info(time_info: &tak_core::TakTimeInfo) -> Self {
        Self {
            white_remaining_ms: time_info.white_remaining.as_millis() as u64,
            black_remaining_ms: time_info.black_remaining.as_millis() as u64,
        }
    }
    fn to_time_info(&self) -> TakTimeInfo {
        TakTimeInfo {
            white_remaining: Duration::from_millis(self.white_remaining_ms),
            black_remaining: Duration::from_millis(self.black_remaining_ms),
        }
    }
}

impl JsonEventRecordType {
    fn from_game_event(event: GameEventType) -> Self {
        match event {
            GameEventType::Action { action } => JsonEventRecordType::Action { action },
            GameEventType::RequestSet { request, player } => JsonEventRecordType::RequestSet {
                request_type: match request {
                    GameRequest::Draw(offer) => JsonRequest::Draw { offer },
                    GameRequest::Undo(request) => JsonRequest::Undo { request },
                    GameRequest::MoreTime(duration) => JsonRequest::MoreTime {
                        amount_ms: duration.map(|d| d.as_millis() as u64),
                    },
                },
                request_player: JsonTakPlayer::from_tak_player(player),
            },
            GameEventType::ActionUndone => JsonEventRecordType::ActionUndone,
            GameEventType::GameOver(game_over_type) => JsonEventRecordType::GameOver {
                game_over_type: match game_over_type {
                    GameOverEventType::Action => JsonEventGameOverType::Action,
                    GameOverEventType::DrawAgreement => JsonEventGameOverType::DrawAgreement,
                    GameOverEventType::Timeout => JsonEventGameOverType::Timeout,
                    GameOverEventType::Resignation => JsonEventGameOverType::Resignation,
                    GameOverEventType::Abandonment => JsonEventGameOverType::Abandonment,
                    GameOverEventType::Aborted => JsonEventGameOverType::Aborted,
                },
            },
            GameEventType::TimeGiven { player, duration } => JsonEventRecordType::TimeGiven {
                player: JsonTakPlayer::from_tak_player(player),
                amount_ms: duration.as_millis() as u64,
            },
        }
    }

    fn to_game_event(&self) -> GameEventType {
        match self {
            JsonEventRecordType::Action { action } => GameEventType::Action {
                action: action.clone(),
            },
            JsonEventRecordType::RequestSet {
                request_type,
                request_player,
            } => GameEventType::RequestSet {
                request: match request_type {
                    JsonRequest::Draw { offer } => GameRequest::Draw(*offer),
                    JsonRequest::Undo { request } => GameRequest::Undo(*request),
                    JsonRequest::MoreTime { amount_ms } => {
                        GameRequest::MoreTime(amount_ms.as_ref().map(|x| Duration::from_millis(*x)))
                    }
                },
                player: request_player.to_tak_player(),
            },
            JsonEventRecordType::ActionUndone => GameEventType::ActionUndone,
            JsonEventRecordType::TimeGiven { player, amount_ms } => GameEventType::TimeGiven {
                player: player.to_tak_player(),
                duration: Duration::from_millis(*amount_ms),
            },
            JsonEventRecordType::GameOver { game_over_type } => {
                GameEventType::GameOver(match game_over_type {
                    JsonEventGameOverType::Action => GameOverEventType::Action,
                    JsonEventGameOverType::DrawAgreement => GameOverEventType::DrawAgreement,
                    JsonEventGameOverType::Timeout => GameOverEventType::Timeout,
                    JsonEventGameOverType::Resignation => GameOverEventType::Resignation,
                    JsonEventGameOverType::Abandonment => GameOverEventType::Abandonment,
                    JsonEventGameOverType::Aborted => GameOverEventType::Aborted,
                })
            }
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum JsonRequest {
    Draw { offer: bool },
    Undo { request: bool },
    MoreTime { amount_ms: Option<u64> },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum JsonTakPlayer {
    White,
    Black,
}
impl JsonTakPlayer {
    fn to_tak_player(&self) -> TakPlayer {
        match self {
            JsonTakPlayer::White => TakPlayer::White,
            JsonTakPlayer::Black => TakPlayer::Black,
        }
    }

    fn from_tak_player(player: TakPlayer) -> Self {
        match player {
            TakPlayer::White => JsonTakPlayer::White,
            TakPlayer::Black => JsonTakPlayer::Black,
        }
    }
}

fn serialize_action<S>(action: &TakAction, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let ptn_string = action_to_ptn(action);
    serializer.serialize_str(&ptn_string)
}

fn deserialize_action<'de, D>(deserializer: D) -> Result<TakAction, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    action_from_ptn(&s).ok_or_else(|| serde::de::Error::custom("Invalid action PTN string"))
}

impl GameRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
        Self { db }
    }

    fn model_to_game(model: game::Model) -> Result<GameRecord, String> {
        let rating_info = if let Some(rating_change_white) = model.rating_change_white
            && let Some(rating_change_black) = model.rating_change_black
        {
            Some(GameRatingInfo {
                rating_change_white: rating_change_white,
                rating_change_black: rating_change_black,
            })
        } else {
            None
        };

        let base_settings = TakBaseGameSettings {
            board_size: model.size as u32,
            half_komi: model.half_komi as u32,
            reserve: TakReserve::new(model.pieces as u32, model.capstones as u32),
            opening: tak_opening_from_string(&model.opening)
                .ok_or_else(|| format!("Invalid opening string in database: {}", model.opening))?,
        };

        let time_settings =
            match serde_json::from_str::<JsonTimeSettings>(&model.game_settings.to_string()) {
                Ok(settings) => settings.to_time_settings(),
                Err(e) => {
                    return Err(format!(
                        "Failed to deserialize game settings from database: {}",
                        e
                    ));
                }
            };
        let json_events: Vec<JsonEventRecord> =
            serde_json::from_value(model.events).unwrap_or_default();

        let white_id = PlayerId(model.player_white_id);
        let black_id = PlayerId(model.player_black_id);

        let white_snapshot = PlayerSnapshot::new(
            model.player_white_username.clone(),
            model.player_white_rating,
        );

        let black_snapshot = PlayerSnapshot::new(
            model.player_black_username.clone(),
            model.player_black_rating,
        );

        let metadata = GameMetadata {
            date: model.date,
            white_id,
            black_id,
            is_rated: model.is_rated,
            settings: TakGameSettings {
                base: base_settings.clone(),
                time_settings: time_settings.clone(),
            },
            match_id: model.match_id.map(|id| MatchId(id)),
        };

        Ok(GameRecord {
            metadata,
            white: white_snapshot,
            black: black_snapshot,
            events: json_events
                .into_iter()
                .map(|jm| GameEvent {
                    date: jm.timestamp,
                    event_type: jm.event.to_game_event(),
                    time_info: jm.time_info.to_time_info(),
                })
                .collect(),
            rating_info,
            result: model
                .result
                .as_deref()
                .and_then(|x| game_result_from_string(x)),
        })
    }
}

#[async_trait::async_trait]
impl GameRepository for GameRepositoryImpl {
    async fn save_ongoing_game(&self, game: GameRecord) -> Result<GameId, RepoError> {
        let time_settings =
            JsonTimeSettings::from_time_settings(&game.metadata.settings.time_settings);

        let base_settings = &game.metadata.settings.base;
        let new_game = game::ActiveModel {
            id: Default::default(), // Auto-increment
            date: Set(game.metadata.date.clone()),
            size: Set(base_settings.board_size as i32),
            player_white_id: Set(game.metadata.white_id.0),
            player_black_id: Set(game.metadata.black_id.0),
            player_white_username: Set(game.white.username),
            player_black_username: Set(game.black.username),
            player_white_rating: Set(game.white.rating),
            player_black_rating: Set(game.black.rating),
            events: Set(serde_json::json!([])),
            result: Set(None),
            is_rated: Set(game.metadata.is_rated),
            half_komi: Set(base_settings.half_komi as i32),
            pieces: Set(base_settings.reserve.pieces as i32),
            capstones: Set(base_settings.reserve.capstones as i32),
            opening: Set(tak_opening_to_string(&base_settings.opening)),
            rating_change_white: Set(None),
            rating_change_black: Set(None),
            game_settings: Set(serde_json::to_value(&time_settings).map_err(|e| {
                RepoError::StorageError(format!("Failed to serialize game settings: {}", e))
            })?),
            match_id: Set(game.metadata.match_id.map(|id| id.0)),
        };

        let result = new_game
            .insert(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;

        Ok(GameId(result.id))
    }

    async fn update_finished_game(
        &self,
        game_id: GameId,
        update: GameFinishedUpdate,
    ) -> Result<(), RepoRetrieveError> {
        let events = update
            .events
            .iter()
            .map(|event| JsonEventRecord {
                timestamp: event.date,
                event: JsonEventRecordType::from_game_event(event.event_type.clone()),
                time_info: JsonTimeInfo::from_time_info(&event.time_info),
            })
            .collect::<Vec<_>>();
        let events = serde_json::to_value(&events)
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?;

        let result_val = game_result_to_string(&update.result);

        let model = game::ActiveModel {
            id: Set(game_id.0),
            events: Set(events),
            result: Set(Some(result_val)),
            rating_change_white: Set(update
                .rating_info
                .as_ref()
                .map(|info| info.rating_change_white)),
            rating_change_black: Set(update
                .rating_info
                .as_ref()
                .map(|info| info.rating_change_black)),

            ..Default::default()
        };

        model
            .update(&self.db)
            .await
            .map_err(|e| db_error_to_repo_retrieve_error(e))?;

        Ok(())
    }

    async fn get_game_record(&self, id: GameId) -> Result<GameRecord, RepoRetrieveError> {
        let model = game::Entity::find_by_id(id.0)
            .one(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?
            .ok_or(RepoRetrieveError::NotFound)?;
        Self::model_to_game(model).map_err(|e| RepoRetrieveError::StorageError(e.to_string()))
    }

    async fn query_games(
        &self,
        filter: GameQuery,
    ) -> Result<PaginatedResponse<(GameId, GameRecord)>, RepoError> {
        let mut query = game::Entity::find();
        if let Some(game_id_selector) = filter.id_selector {
            query = match game_id_selector {
                GameIdSelector::Range(start_id, end_id) => {
                    query.filter(game::Column::Id.between(start_id.0, end_id.0))
                }
                GameIdSelector::AndBefore(end_id) => query.filter(game::Column::Id.lte(end_id.0)),
                GameIdSelector::AndAfter(start_id) => {
                    query.filter(game::Column::Id.gte(start_id.0))
                }
                GameIdSelector::List(id_list) => {
                    query.filter(game::Column::Id.is_in(id_list.iter().map(|id| id.0)))
                }
            }
        }
        if let Some(date_selector) = filter.date_selector {
            query = match date_selector {
                DateSelector::Range(start_date, end_date) => query.filter(
                    game::Column::Date.between(start_date.timestamp(), end_date.timestamp()),
                ),
                DateSelector::Before(end_date) => {
                    query.filter(game::Column::Date.lte(end_date.timestamp()))
                }
                DateSelector::After(start_date) => {
                    query.filter(game::Column::Date.gte(start_date.timestamp()))
                }
            }
        }
        for (filter, color) in filter.player_filters {
            let condition =
                match filter {
                    GamePlayerFilter::Contains(name_part) => {
                        let condition = sea_orm::Condition::any();
                        match color {
                            Some(TakPlayer::White) => condition
                                .add(game::Column::PlayerWhiteUsername.contains(&name_part)),
                            Some(TakPlayer::Black) => condition
                                .add(game::Column::PlayerBlackUsername.contains(&name_part)),
                            None => condition
                                .add(game::Column::PlayerWhiteUsername.contains(&name_part))
                                .add(game::Column::PlayerBlackUsername.contains(&name_part)),
                        }
                    }
                    GamePlayerFilter::Equals(name) => {
                        let condition = sea_orm::Condition::any();
                        match color {
                            Some(TakPlayer::White) => {
                                condition.add(game::Column::PlayerWhiteUsername.eq(&name))
                            }
                            Some(TakPlayer::Black) => {
                                condition.add(game::Column::PlayerBlackUsername.eq(&name))
                            }
                            None => condition
                                .add(game::Column::PlayerWhiteUsername.eq(&name))
                                .add(game::Column::PlayerBlackUsername.eq(&name)),
                        }
                    }
                    GamePlayerFilter::PlayerId(player_id) => {
                        let condition = sea_orm::Condition::any();
                        match color {
                            Some(TakPlayer::White) => {
                                condition.add(game::Column::PlayerWhiteId.eq(player_id.0))
                            }
                            Some(TakPlayer::Black) => {
                                condition.add(game::Column::PlayerBlackId.eq(player_id.0))
                            }
                            None => condition
                                .add(game::Column::PlayerWhiteId.eq(player_id.0))
                                .add(game::Column::PlayerBlackId.eq(player_id.0)),
                        }
                    }
                };
            query = query.filter(condition);
        }

        if let Some(is_rated) = filter.is_rated {
            query = query.filter(game::Column::IsRated.eq(is_rated));
        }

        if let Some(game_results) = filter.game_results {
            let result_strings: Vec<String> = game_results
                .iter()
                .map(|result| game_result_to_string(result))
                .collect();
            query = query.filter(game::Column::Result.is_in(result_strings));
        } else {
            query = query.filter(game::Column::Result.is_not_null());
        }

        if let Some(half_komi) = filter.half_komi {
            query = query.filter(game::Column::HalfKomi.eq(half_komi as i32));
        }
        if let Some(board_size) = filter.board_size {
            query = query.filter(game::Column::Size.eq(board_size as i32));
        }

        let total_count: u64 = query
            .clone()
            .count(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;

        if let Some((sort_order, sort_by)) = filter.sort {
            query = match (sort_by, sort_order) {
                (GameSortBy::Date, SortOrder::Ascending) => query.order_by_asc(game::Column::Date),
                (GameSortBy::Date, SortOrder::Descending) => {
                    query.order_by_desc(game::Column::Date)
                }
                (GameSortBy::GameId, SortOrder::Ascending) => query.order_by_asc(game::Column::Id),
                (GameSortBy::GameId, SortOrder::Descending) => {
                    query.order_by_desc(game::Column::Id)
                }
            }
        }

        if let Some(offset) = filter.pagination.offset {
            query = query.offset(offset as u64);
        }
        if let Some(limit) = filter.pagination.limit {
            query = query.limit(limit as u64);
        }

        let models = query
            .all(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;

        let mut results = Vec::new();
        for model in models {
            let game_id = GameId(model.id);
            let game_record = Self::model_to_game(model).map_err(|e| {
                RepoError::StorageError(format!("Failed to convert game model to record: {}", e))
            })?;
            results.push((game_id, game_record));
        }

        Ok(PaginatedResponse {
            total_count: total_count as usize,
            items: results,
        })
    }

    async fn get_games_of_match(
        &self,
        match_id: MatchId,
    ) -> Result<Vec<(GameId, GameRecord)>, RepoError> {
        let models = game::Entity::find()
            .filter(game::Column::MatchId.eq(match_id.0))
            .all(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;

        let mut results = Vec::new();
        for model in models {
            let game_id = GameId(model.id);
            let game_record = Self::model_to_game(model).map_err(|e| {
                RepoError::StorageError(format!("Failed to convert game model to record: {}", e))
            })?;
            results.push((game_id, game_record));
        }

        Ok(results)
    }
}
