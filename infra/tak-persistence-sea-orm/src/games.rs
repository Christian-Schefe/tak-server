use std::time::Duration;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use serde::Deserialize;
use tak_core::{
    TakAction, TakGameSettings, TakPlayer, TakRequestType, TakReserve, TakTimeControl,
    ptn::{action_from_ptn, action_to_ptn, game_result_from_string, game_result_to_string},
};
use tak_persistence_sea_orm_entites::game;
use tak_server_app::domain::{
    GameId, PaginatedResponse, PlayerId, RepoError, RepoRetrieveError, RepoUpdateError, SortOrder,
    game::{GameEvent, GameEventType},
    game_history::{
        DateSelector, GameFinishedUpdate, GameIdSelector, GamePlayerFilter, GameQuery,
        GameRatingInfo, GameRecord, GameRepository, GameSortBy, PlayerSnapshot,
    },
};

use crate::create_db_pool;

pub struct GameRepositoryImpl {
    db: DatabaseConnection,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct JsonEventRecord {
    timestamp: chrono::DateTime<chrono::Utc>,
    event: JsonEventRecordType,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
enum JsonEventRecordType {
    Action {
        #[serde(
            serialize_with = "serialize_action",
            deserialize_with = "deserialize_action"
        )]
        action: TakAction,
        white_remaining_ms: u64,
        black_remaining_ms: u64,
    },
    RequestAdded {
        request_id: u64,
        request_type: JsonRequestType,
        request_player: JsonTakPlayer,
    },
    RequestRetracted {
        request_id: u64,
    },
    RequestRejected {
        request_id: u64,
    },
    RequestAccepted {
        request_id: u64,
    },
    ActionUndone,
    DrawAgreed,
    TimeGiven {
        player: JsonTakPlayer,
        amount_ms: u64,
    },
    Timeout,
    Resigned {
        player: JsonTakPlayer,
    },
}

impl JsonEventRecordType {
    fn from_game_event(event: GameEventType) -> Self {
        match event {
            GameEventType::Action {
                action,
                white_remaining,
                black_remaining,
            } => JsonEventRecordType::Action {
                action,
                white_remaining_ms: white_remaining.as_millis() as u64,
                black_remaining_ms: black_remaining.as_millis() as u64,
            },
            GameEventType::RequestAdded { request } => JsonEventRecordType::RequestAdded {
                request_id: request.id.0,
                request_type: match request.request_type {
                    TakRequestType::Draw => JsonRequestType::Draw,
                    TakRequestType::Undo => JsonRequestType::Undo,
                    TakRequestType::MoreTime(duration) => JsonRequestType::MoreTime {
                        amount_ms: duration.as_millis() as u64,
                    },
                },
                request_player: JsonTakPlayer::from_tak_player(&request.player),
            },
            GameEventType::RequestRetracted { request_id } => {
                JsonEventRecordType::RequestRetracted {
                    request_id: request_id.0,
                }
            }
            GameEventType::RequestRejected { request_id } => JsonEventRecordType::RequestRejected {
                request_id: request_id.0,
            },
            GameEventType::RequestAccepted { request_id } => JsonEventRecordType::RequestAccepted {
                request_id: request_id.0,
            },
            GameEventType::ActionUndone => JsonEventRecordType::ActionUndone,
            GameEventType::DrawAgreed => JsonEventRecordType::DrawAgreed,
            GameEventType::TimeGiven { player, duration } => JsonEventRecordType::TimeGiven {
                player: JsonTakPlayer::from_tak_player(&player),
                amount_ms: duration.as_millis() as u64,
            },
            GameEventType::Timeout => todo!(),
            GameEventType::Resigned(tak_player) => JsonEventRecordType::Resigned {
                player: JsonTakPlayer::from_tak_player(&tak_player),
            },
        }
    }

    fn to_game_event(&self) -> GameEventType {
        match self {
            JsonEventRecordType::Action {
                action,
                white_remaining_ms,
                black_remaining_ms,
            } => GameEventType::Action {
                action: action.clone(),
                white_remaining: Duration::from_millis(*white_remaining_ms),
                black_remaining: Duration::from_millis(*black_remaining_ms),
            },
            JsonEventRecordType::RequestAdded {
                request_id,
                request_type,
                request_player,
            } => GameEventType::RequestAdded {
                request: tak_core::TakRequest {
                    id: tak_core::TakRequestId(*request_id),
                    request_type: match request_type {
                        JsonRequestType::Draw => TakRequestType::Draw,
                        JsonRequestType::Undo => TakRequestType::Undo,
                        JsonRequestType::MoreTime { amount_ms } => {
                            TakRequestType::MoreTime(Duration::from_millis(*amount_ms))
                        }
                    },
                    player: request_player.to_tak_player(),
                },
            },
            JsonEventRecordType::RequestRetracted { request_id } => {
                GameEventType::RequestRetracted {
                    request_id: tak_core::TakRequestId(*request_id),
                }
            }
            JsonEventRecordType::RequestRejected { request_id } => GameEventType::RequestRejected {
                request_id: tak_core::TakRequestId(*request_id),
            },
            JsonEventRecordType::RequestAccepted { request_id } => GameEventType::RequestAccepted {
                request_id: tak_core::TakRequestId(*request_id),
            },
            JsonEventRecordType::ActionUndone => GameEventType::ActionUndone,
            JsonEventRecordType::DrawAgreed => GameEventType::DrawAgreed,
            JsonEventRecordType::TimeGiven { player, amount_ms } => GameEventType::TimeGiven {
                player: player.to_tak_player(),
                duration: Duration::from_millis(*amount_ms),
            },
            JsonEventRecordType::Timeout => GameEventType::Timeout,
            JsonEventRecordType::Resigned { player } => {
                GameEventType::Resigned(player.to_tak_player())
            }
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
enum JsonRequestType {
    Draw,
    Undo,
    MoreTime { amount_ms: u64 },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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

    fn from_tak_player(player: &TakPlayer) -> Self {
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

    fn db_error_to_repo_error(e: sea_orm::DbErr) -> RepoUpdateError {
        match e {
            sea_orm::DbErr::RecordNotFound(_) | sea_orm::DbErr::RecordNotUpdated => {
                RepoUpdateError::NotFound
            }
            e => match e.sql_err() {
                Some(
                    sea_orm::SqlErr::UniqueConstraintViolation(_)
                    | sea_orm::SqlErr::ForeignKeyConstraintViolation(_),
                ) => RepoUpdateError::Conflict,
                _ => RepoUpdateError::StorageError(e.to_string()),
            },
        }
    }

    fn model_to_game(model: game::Model) -> GameRecord {
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
        let time_control = TakTimeControl {
            contingent: Duration::from_millis(model.clock_contingent_ms as u64),
            increment: Duration::from_millis(model.clock_increment_ms as u64),
            extra: if model.extra_time_amount > 0 && model.extra_time_trigger > 0 {
                Some((
                    model.extra_time_amount as u32,
                    Duration::from_secs(model.extra_time_trigger as u64),
                ))
            } else {
                None
            },
        };
        let json_events: Vec<JsonEventRecord> =
            serde_json::from_value(model.events).unwrap_or_default();

        let white_snapshot = PlayerSnapshot::new(
            PlayerId(model.player_white_id),
            model.player_white_username.clone(),
            model.player_white_rating,
        );

        let black_snapshot = PlayerSnapshot::new(
            PlayerId(model.player_black_id),
            model.player_black_username.clone(),
            model.player_black_rating,
        );

        GameRecord {
            date: model.date.clone(),
            settings: TakGameSettings {
                board_size: model.size as u32,
                time_control,
                half_komi: model.half_komi as u32,
                reserve: TakReserve::new(model.pieces as u32, model.capstones as u32),
            },
            white: white_snapshot,
            black: black_snapshot,
            is_rated: model.is_rated,
            events: json_events
                .into_iter()
                .map(|jm| GameEvent {
                    date: jm.timestamp,
                    event_type: jm.event.to_game_event(),
                })
                .collect(),
            rating_info,
            result: model
                .result
                .as_deref()
                .and_then(|x| game_result_from_string(x)),
        }
    }
}

#[async_trait::async_trait]
impl GameRepository for GameRepositoryImpl {
    async fn save_ongoing_game(&self, game: GameRecord) -> Result<GameId, RepoError> {
        let new_game = game::ActiveModel {
            id: Default::default(), // Auto-increment
            date: Set(game.date.clone()),
            size: Set(game.settings.board_size as i32),
            player_white_id: Set(game.white.player_id.0),
            player_black_id: Set(game.black.player_id.0),
            player_white_username: Set(game.white.username),
            player_black_username: Set(game.black.username),
            player_white_rating: Set(game.white.rating),
            player_black_rating: Set(game.black.rating),
            events: Set(serde_json::json!([])),
            result: Set(None),
            clock_contingent_ms: Set(game.settings.time_control.contingent.as_millis() as i64),
            clock_increment_ms: Set(game.settings.time_control.increment.as_millis() as i64),
            is_rated: Set(game.is_rated),
            half_komi: Set(game.settings.half_komi as i32),
            pieces: Set(game.settings.reserve.pieces as i32),
            capstones: Set(game.settings.reserve.capstones as i32),
            rating_change_white: Set(None),
            rating_change_black: Set(None),
            extra_time_amount: Set(game
                .settings
                .time_control
                .extra
                .as_ref()
                .map_or(0, |(trigger_move, _)| *trigger_move as i32)),
            extra_time_trigger: Set(game
                .settings
                .time_control
                .extra
                .as_ref()
                .map_or(0, |(_, extra_time)| extra_time.as_secs() as i32)),
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
    ) -> Result<(), RepoUpdateError> {
        let events = update
            .events
            .iter()
            .map(|event| JsonEventRecord {
                timestamp: event.date,
                event: JsonEventRecordType::from_game_event(event.event_type.clone()),
            })
            .collect::<Vec<_>>();
        let events = serde_json::to_value(&events)
            .map_err(|e| RepoUpdateError::StorageError(e.to_string()))?;

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
            .map_err(Self::db_error_to_repo_error)?;

        Ok(())
    }

    async fn get_game_record(&self, id: GameId) -> Result<GameRecord, RepoRetrieveError> {
        let model = game::Entity::find_by_id(id.0)
            .one(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?
            .ok_or(RepoRetrieveError::NotFound)?;
        Ok(Self::model_to_game(model))
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
        if let Some(player_white) = filter.player_white {
            query = match player_white {
                GamePlayerFilter::Contains(name_part) => {
                    query.filter(game::Column::PlayerWhiteUsername.contains(&name_part))
                }
                GamePlayerFilter::Equals(name) => {
                    query.filter(game::Column::PlayerWhiteUsername.eq(name))
                }
            };
        }
        if let Some(player_black) = filter.player_black {
            query = match player_black {
                GamePlayerFilter::Contains(name_part) => {
                    query.filter(game::Column::PlayerBlackUsername.contains(&name_part))
                }
                GamePlayerFilter::Equals(name) => {
                    query.filter(game::Column::PlayerBlackUsername.eq(name))
                }
            };
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
        }
        if let Some(half_komi) = filter.half_komi {
            query = query.filter(game::Column::HalfKomi.eq(half_komi as i32));
        }
        if let Some(board_size) = filter.board_size {
            query = query.filter(game::Column::Size.eq(board_size as i32));
        }

        if let Some(clock_contingent) = filter.clock_contingent {
            query = query
                .filter(game::Column::ClockContingentMs.eq(clock_contingent.as_millis() as i64));
        }
        if let Some(clock_increment) = filter.clock_increment {
            query =
                query.filter(game::Column::ClockIncrementMs.eq(clock_increment.as_millis() as i64));
        }
        if let Some(clock_extra_time) = filter.clock_extra_time {
            query =
                query.filter(game::Column::ExtraTimeAmount.eq(clock_extra_time.as_secs() as i32));
        }
        if let Some(clock_extra_trigger) = filter.clock_extra_trigger {
            query = query.filter(game::Column::ExtraTimeTrigger.eq(clock_extra_trigger as i32));
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
            let game_record = Self::model_to_game(model);
            results.push((game_id, game_record));
        }

        Ok(PaginatedResponse {
            total_count: total_count as usize,
            items: results,
        })
    }
}
