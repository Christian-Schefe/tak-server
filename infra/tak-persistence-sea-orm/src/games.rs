use std::time::Duration;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use serde::Deserialize;
use tak_core::{
    TakAction, TakActionRecord, TakGameSettings, TakGameState, TakReserve, TakTimeControl,
    ptn::{action_from_ptn, action_to_ptn, game_state_from_string, game_state_to_string},
};
use tak_server_app::domain::{
    FinishedGameId, GameType, PlayerId, SortOrder,
    game_history::{
        DateSelector, GameFilter, GameFilterResult, GameFinishedUpdate, GameIdSelector,
        GamePlayerFilter, GameRatingInfo, GameRecord, GameRepoError, GameRepository, GameSortBy,
        PlayerSnapshot, ReadGameError,
    },
};

use crate::{create_games_db_pool, entity::game};

pub struct GameRepositoryImpl {
    db: DatabaseConnection,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct JsonMoveRecord {
    #[serde(
        serialize_with = "serialize_action",
        deserialize_with = "deserialize_action"
    )]
    pub action: TakAction,
    pub white_remaining_ms: u64,
    pub black_remaining_ms: u64,
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
        let db = create_games_db_pool().await;
        Self { db }
    }

    fn model_to_game(model: &game::Model) -> GameRecord {
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
            contingent: Duration::from_secs(model.clock_contingent as u64),
            increment: Duration::from_secs(model.clock_increment as u64),
            extra: if model.extra_time_amount > 0 && model.extra_time_trigger > 0 {
                Some((
                    model.extra_time_amount as u32,
                    Duration::from_secs(model.extra_time_trigger as u64),
                ))
            } else {
                None
            },
        };
        let json_moves: Vec<JsonMoveRecord> =
            serde_json::from_str(&model.notation).unwrap_or_default();

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
            game_type: if model.is_unrated {
                GameType::Unrated
            } else if model.is_tournament {
                GameType::Tournament
            } else {
                GameType::Rated
            },
            moves: json_moves
                .into_iter()
                .map(|jm| TakActionRecord {
                    action: jm.action,
                    time_remaining: (
                        Duration::from_millis(jm.white_remaining_ms),
                        Duration::from_millis(jm.black_remaining_ms),
                    ),
                })
                .collect(),
            rating_info,
            result: game_state_from_string(&model.result).unwrap_or(TakGameState::Ongoing),
        }
    }
}

#[async_trait::async_trait]
impl GameRepository for GameRepositoryImpl {
    async fn save_ongoing_game(&self, game: GameRecord) -> Result<FinishedGameId, GameRepoError> {
        let new_game = game::ActiveModel {
            id: Default::default(), // Auto-increment
            date: Set(game.date.clone()),
            size: Set(game.settings.board_size as i32),
            player_white_id: Set(game.white.player_id.0),
            player_black_id: Set(game.black.player_id.0),
            player_white_username: Set(game.white.username.clone()),
            player_black_username: Set(game.black.username.clone()),
            player_white_rating: Set(game.white.rating),
            player_black_rating: Set(game.black.rating),
            notation: Set(String::new()),
            result: Set("0-0".to_string()),
            clock_contingent: Set(game.settings.time_control.contingent.as_secs() as i32),
            clock_increment: Set(game.settings.time_control.increment.as_secs() as i32),
            is_unrated: Set(game.game_type == GameType::Unrated),
            is_tournament: Set(game.game_type == GameType::Tournament),
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
            .map_err(|e| GameRepoError::StorageError(e.to_string()))?;

        Ok(FinishedGameId(result.id))
    }

    async fn update_finished_game(
        &self,
        game_id: FinishedGameId,
        update: GameFinishedUpdate,
    ) -> Result<(), GameRepoError> {
        let notation_val = update
            .moves
            .iter()
            .map(|action| JsonMoveRecord {
                action: action.action.clone(),
                white_remaining_ms: action.time_remaining.0.as_millis() as u64,
                black_remaining_ms: action.time_remaining.1.as_millis() as u64,
            })
            .collect::<Vec<_>>();
        let notation_str = serde_json::to_string(&notation_val)
            .map_err(|e| GameRepoError::StorageError(e.to_string()))?;

        let result_val = game_state_to_string(&update.result);

        let game = game::Entity::find_by_id(game_id.0)
            .one(&self.db)
            .await
            .map_err(|e| GameRepoError::StorageError(e.to_string()))?
            .ok_or_else(|| GameRepoError::StorageError("Game not found".to_string()))?;

        let mut game: game::ActiveModel = game.into();
        game.notation = Set(notation_str);
        game.result = Set(result_val);
        game.player_white_rating = Set(update.player_white.rating);
        game.player_black_rating = Set(update.player_black.rating);
        game.rating_change_white = Set(update
            .rating_info
            .as_ref()
            .map(|info| info.rating_change_white));
        game.rating_change_black = Set(update
            .rating_info
            .as_ref()
            .map(|info| info.rating_change_black));

        game.update(&self.db)
            .await
            .map_err(|e| GameRepoError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn get_game_record(&self, id: FinishedGameId) -> Result<GameRecord, ReadGameError> {
        let model = game::Entity::find_by_id(id.0)
            .one(&self.db)
            .await
            .map_err(|e| ReadGameError::StorageError(e.to_string()))?
            .ok_or(ReadGameError::NotFound)?;
        Ok(Self::model_to_game(&model))
    }

    async fn get_games(&self, filter: GameFilter) -> Result<GameFilterResult, GameRepoError> {
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
        if let Some(game_type) = filter.game_type {
            query = match game_type {
                GameType::Rated => query
                    .filter(game::Column::IsUnrated.eq(false))
                    .filter(game::Column::IsTournament.eq(false)),
                GameType::Unrated => query.filter(game::Column::IsUnrated.eq(true)),
                GameType::Tournament => query.filter(game::Column::IsTournament.eq(true)),
            }
        }
        if let Some(game_states) = filter.game_states {
            let state_strings: Vec<String> = game_states
                .iter()
                .map(|state| game_state_to_string(state))
                .collect();
            query = query.filter(game::Column::Result.is_in(state_strings));
        }
        if let Some(half_komi) = filter.half_komi {
            query = query.filter(game::Column::HalfKomi.eq(half_komi as i32));
        }
        if let Some(board_size) = filter.board_size {
            query = query.filter(game::Column::Size.eq(board_size as i32));
        }

        if let Some(clock_contingent) = filter.clock_contingent {
            query =
                query.filter(game::Column::ClockContingent.eq(clock_contingent.as_secs() as i32));
        }
        if let Some(clock_increment) = filter.clock_increment {
            query = query.filter(game::Column::ClockIncrement.eq(clock_increment.as_secs() as i32));
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
            .map_err(|e| GameRepoError::StorageError(e.to_string()))?;

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
            .map_err(|e| GameRepoError::StorageError(e.to_string()))?;

        let mut results = Vec::new();
        for model in models {
            let game_record = Self::model_to_game(&model);
            results.push((FinishedGameId(model.id), game_record));
        }

        Ok(GameFilterResult {
            total_count: total_count as usize,
            games: results,
        })
    }
}
