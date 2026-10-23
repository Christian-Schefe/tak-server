use std::time::{Duration, UNIX_EPOCH};

use sea_orm::{
    ColumnTrait, EntityTrait, ExprTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use tak_core::{
    TakActionRecord, TakBaseGameSettings, TakClock, TakGame, TakGameSettings, TakPlayer,
    TakReserve, TakTimeInfo,
    ptn::{action_from_ptn, action_to_ptn, game_result_from_string, game_result_to_string},
};
use tak_domain::{
    GameId, MatchId, UserId,
    game::{Game, request::GameRequests},
    game_record::{GameMetadata, GameRatingInfo, GameRecord, PlayerSnapshot},
};
use tak_persistence_sea_orm_entities::game;
use tak_server_app::ports::{
    PaginatedResponse, SortOrder,
    game::{DateSelector, GameIdSelector, GamePlayerFilter, GameQuery, GameRepository, GameSortBy},
    transaction::RepoError,
};

use crate::{
    JsonTimeSettings, tak_opening_from_string, tak_opening_to_string,
    transaction::RepositoryContext,
};

pub struct GameRepositoryImpl;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonActionRecord {
    pub action: String,
    pub time_info: JsonTimeInfo,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonTimeInfo {
    pub white_remaining_ms: u64,
    pub black_remaining_ms: u64,
}

impl JsonTimeInfo {
    pub fn from_time_info(time_info: &TakTimeInfo) -> Self {
        Self {
            white_remaining_ms: time_info.white_remaining.as_millis() as u64,
            black_remaining_ms: time_info.black_remaining.as_millis() as u64,
        }
    }
    pub fn to_time_info(&self) -> TakTimeInfo {
        TakTimeInfo {
            white_remaining: Duration::from_millis(self.white_remaining_ms),
            black_remaining: Duration::from_millis(self.black_remaining_ms),
        }
    }
}

impl GameRepositoryImpl {
    fn model_to_game(model: game::Model) -> Result<(GameId, GameRecord), String> {
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

        let white_id = UserId(model.player_white_id);
        let black_id = UserId(model.player_black_id);

        let white_snapshot = PlayerSnapshot::new(
            white_id,
            model.player_white_username.clone(),
            model.player_white_rating,
        );

        let black_snapshot = PlayerSnapshot::new(
            black_id,
            model.player_black_username.clone(),
            model.player_black_rating,
        );

        let metadata = GameMetadata {
            date: model.date,
            white: white_snapshot,
            black: black_snapshot,
            is_rated: model.is_rated,
            settings: TakGameSettings {
                base: base_settings.clone(),
                time_settings: time_settings.clone(),
            },
            match_id: model.match_id.map(|id| MatchId(id)),
        };

        let history: Vec<TakActionRecord> =
            serde_json::from_value::<Vec<JsonActionRecord>>(model.history)
                .map_err(|e| format!("Failed to deserialize game history from JSON: {}", e))?
                .into_iter()
                .map(|json_action_record: JsonActionRecord| {
                    Ok(TakActionRecord {
                        action: action_from_ptn(&json_action_record.action)
                            .ok_or_else(|| format!("Failed to parse action from PTN"))?,
                        time_info: json_action_record.time_info.to_time_info(),
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;

        let clock = TakClock {
            remaining_time: TakTimeInfo {
                white_remaining: Duration::from_millis(model.white_remaining_ms as u64),
                black_remaining: Duration::from_millis(model.black_remaining_ms as u64),
            },
            last_update_timestamp: UNIX_EPOCH
                + Duration::from_millis(model.last_clock_update.timestamp_millis() as u64),
            is_ticking: model.is_clock_ticking,
        };

        let result = model
            .result
            .as_ref()
            .map(|res_str| {
                game_result_from_string(res_str)
                    .ok_or_else(|| format!("Invalid game result string in database: {}", res_str))
            })
            .transpose()?;

        let game = TakGame::from_history(&metadata.settings, clock, history, result)
            .map_err(|e| format!("Failed to reconstruct game from history: {:?}", e))?;

        let white_requests = GameRequests {
            draw_offered: model.white_draw_offer,
            undo_requested: model.white_undo_request,
            more_time_offered: model
                .white_more_time_request
                .map(|ms| Duration::from_millis(ms as u64)),
        };
        let black_requests = GameRequests {
            draw_offered: model.black_draw_offer,
            undo_requested: model.black_undo_request,
            more_time_offered: model
                .black_more_time_request
                .map(|ms| Duration::from_millis(ms as u64)),
        };

        Ok((
            GameId(model.id),
            GameRecord {
                metadata,
                rating_info,
                game: Game {
                    game,
                    white_requests,
                    black_requests,
                },
            },
        ))
    }
    fn game_to_model(
        game_id: Option<GameId>,
        record: &GameRecord,
    ) -> Result<game::ActiveModel, String> {
        let result = record.game.game.result().map(|f| game_result_to_string(f));

        let base_settings = &record.metadata.settings.base;
        let time_settings =
            JsonTimeSettings::from_time_settings(&record.metadata.settings.time_settings);

        let game = &record.game.game;

        let history: Vec<JsonActionRecord> = game
            .history
            .iter()
            .map(|action| JsonActionRecord {
                action: action_to_ptn(&action.action),
                time_info: JsonTimeInfo::from_time_info(&action.time_info),
            })
            .collect();

        let json_history = serde_json::to_value(history)
            .map_err(|e| format!("Failed to serialize game history to JSON: {}", e))?;

        Ok(game::ActiveModel {
            id: game_id.map_or(Default::default(), |id| Set(id.0)),
            history: Set(json_history),
            white_remaining_ms: Set(game.clock.remaining_time.white_remaining.as_millis() as i64),
            black_remaining_ms: Set(game.clock.remaining_time.black_remaining.as_millis() as i64),
            last_clock_update: Set(chrono::DateTime::from_timestamp_millis(
                game.clock
                    .last_update_timestamp
                    .duration_since(UNIX_EPOCH)
                    .map_err(|e| format!("Failed to convert timestamp to chrono::DateTime: {}", e))?
                    .as_millis() as i64,
            )
            .ok_or("Failed to convert timestamp to chrono::DateTime")?),
            is_clock_ticking: Set(game.clock.is_ticking),
            white_draw_offer: Set(record.game.white_requests.draw_offered),
            black_draw_offer: Set(record.game.black_requests.draw_offered),
            white_undo_request: Set(record.game.white_requests.undo_requested),
            black_undo_request: Set(record.game.black_requests.undo_requested),
            white_more_time_request: Set(record
                .game
                .white_requests
                .more_time_offered
                .map(|d| d.as_millis() as i64)),
            black_more_time_request: Set(record
                .game
                .black_requests
                .more_time_offered
                .map(|d| d.as_millis() as i64)),

            date: Set(record.metadata.date),
            size: Set(base_settings.board_size as i32),
            player_white_id: Set(record.metadata.white.user_id.0),
            player_black_id: Set(record.metadata.black.user_id.0),
            player_white_username: Set(record.metadata.white.username.clone()),
            player_black_username: Set(record.metadata.black.username.clone()),
            player_white_rating: Set(record.metadata.white.rating),
            player_black_rating: Set(record.metadata.black.rating),
            result: Set(result),
            is_rated: Set(record.metadata.is_rated),
            half_komi: Set(base_settings.half_komi as i32),
            pieces: Set(base_settings.reserve.pieces as i32),
            capstones: Set(base_settings.reserve.capstones as i32),
            opening: Set(tak_opening_to_string(&base_settings.opening)),
            rating_change_white: Set(record
                .rating_info
                .as_ref()
                .map(|info| info.rating_change_white)),
            rating_change_black: Set(record
                .rating_info
                .as_ref()
                .map(|info| info.rating_change_black)),
            game_settings: Set(serde_json::to_value(&time_settings)
                .map_err(|e| format!("Failed to serialize game settings: {}", e))?),
            match_id: Set(record.metadata.match_id.map(|id| id.0)),
        })
    }
}

#[async_trait::async_trait]
impl GameRepository<RepositoryContext> for GameRepositoryImpl {
    async fn get_game(
        &self,
        ctx: &RepositoryContext,
        game_id: GameId,
    ) -> Result<Option<GameRecord>, RepoError> {
        let model = game::Entity::find_by_id(game_id.0)
            .one(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;

        if let Some(model) = model {
            let (_, game) = Self::model_to_game(model)
                .map_err(|e| RepoError(format!("Failed to convert game model to record: {}", e)))?;
            Ok(Some(game))
        } else {
            Ok(None)
        }
    }
    async fn create_game(
        &self,
        ctx: &RepositoryContext,
        game: GameRecord,
    ) -> Result<GameId, RepoError> {
        let model = Self::game_to_model(None, &game)
            .map_err(|e| RepoError(format!("Failed to convert game record to model: {}", e)))?;
        let res = game::Entity::insert(model)
            .exec(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;
        let game_id = GameId(res.last_insert_id);
        Ok(game_id)
    }
    async fn update_game(
        &self,
        ctx: &RepositoryContext,
        game_id: GameId,
        game: GameRecord,
    ) -> Result<(), RepoError> {
        let model = Self::game_to_model(Some(game_id), &game)
            .map_err(|e| RepoError(format!("Failed to convert game record to model: {}", e)))?;
        game::Entity::update(model)
            .exec(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;
        Ok(())
    }
    async fn get_ongoing_games(
        &self,
        ctx: &RepositoryContext,
    ) -> Result<Vec<(GameId, GameRecord)>, RepoError> {
        let models = game::Entity::find()
            .filter(game::Column::Result.is_null())
            .all(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;

        let mut results = Vec::new();
        for model in models {
            let (game_id, game) = Self::model_to_game(model)
                .map_err(|e| RepoError(format!("Failed to convert game model to record: {}", e)))?;
            results.push((game_id, game));
        }
        Ok(results)
    }
    async fn get_ongoing_games_by_player(
        &self,
        ctx: &RepositoryContext,
        player_id: UserId,
    ) -> Result<Vec<(GameId, GameRecord)>, RepoError> {
        let models = game::Entity::find()
            .filter(
                game::Column::Result.is_null().and(
                    game::Column::PlayerWhiteId
                        .eq(player_id.0)
                        .or(game::Column::PlayerBlackId.eq(player_id.0)),
                ),
            )
            .all(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;

        let mut results = Vec::new();
        for model in models {
            let (game_id, game) = Self::model_to_game(model)
                .map_err(|e| RepoError(format!("Failed to convert game model to record: {}", e)))?;
            results.push((game_id, game));
        }
        Ok(results)
    }
    async fn query_games(
        &self,
        ctx: &RepositoryContext,
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
                    GamePlayerFilter::UserId(user_id) => {
                        let condition = sea_orm::Condition::any();
                        match color {
                            Some(TakPlayer::White) => {
                                condition.add(game::Column::PlayerWhiteId.eq(user_id.0))
                            }
                            Some(TakPlayer::Black) => {
                                condition.add(game::Column::PlayerBlackId.eq(user_id.0))
                            }
                            None => condition
                                .add(game::Column::PlayerWhiteId.eq(user_id.0))
                                .add(game::Column::PlayerBlackId.eq(user_id.0)),
                        }
                    }
                };
            query = query.filter(condition);
        }

        if let Some(is_rated) = filter.is_rated {
            query = query.filter(game::Column::IsRated.eq(is_rated));
        }

        query = query.filter(game::Column::Result.is_not_null());
        if !filter.game_results.is_empty() {
            let result_strings: Vec<String> = filter
                .game_results
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

        let total_count: u64 = query
            .clone()
            .count(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;

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

        let models = query.all(ctx).await.map_err(|e| RepoError(e.to_string()))?;

        let mut results = Vec::new();
        for model in models {
            let game_record = Self::model_to_game(model)
                .map_err(|e| RepoError(format!("Failed to convert game model to record: {}", e)))?;
            results.push(game_record);
        }

        Ok(PaginatedResponse {
            total_count: total_count as usize,
            items: results,
        })
    }

    async fn get_games_of_match(
        &self,
        ctx: &RepositoryContext,
        match_id: MatchId,
    ) -> Result<Vec<(GameId, GameRecord)>, RepoError> {
        let models = game::Entity::find()
            .filter(game::Column::MatchId.eq(match_id.0))
            .all(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;

        let mut results = Vec::new();
        for model in models {
            let game_record = Self::model_to_game(model)
                .map_err(|e| RepoError(format!("Failed to convert game model to record: {}", e)))?;
            results.push(game_record);
        }

        Ok(results)
    }
}
