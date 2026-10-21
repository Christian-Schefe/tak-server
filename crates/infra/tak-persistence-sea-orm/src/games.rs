use std::time::Duration;

use nonempty::NonEmpty;
use sea_orm::{
    ColumnTrait, EntityTrait, ExprTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::Deserialize;
use tak_core::{
    TakAction, TakBaseGameSettings, TakGameSettings, TakPlayer, TakReserve, TakTimeInfo,
    ptn::{action_from_ptn, action_to_ptn, game_result_from_string, game_result_to_string},
};
use tak_domain::{
    GameId, MatchId, UserId,
    game::{
        FinishedGame, Game, GameEvent, GameEventType, GameMetadata, GameOverEventType,
        GameRatingInfo, OngoingGame, PlayerSnapshot, request::GameRequest,
    },
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
    Start,
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
            GameEventType::Start => JsonEventRecordType::Start,
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
            JsonEventRecordType::Start => GameEventType::Start,
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
    fn model_to_game(model: game::Model) -> Result<(GameId, Game), String> {
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

        let Some(events) = NonEmpty::collect(json_events.into_iter().map(|jm| GameEvent {
            date: jm.timestamp,
            event_type: jm.event.to_game_event(),
            time_info: jm.time_info.to_time_info(),
        })) else {
            return Err("Game events list is empty".to_string());
        };

        let game = if let Some(result) = model.result {
            Game::Finished(FinishedGame {
                metadata,
                rating_info,
                events,
                result: game_result_from_string(&result)
                    .ok_or_else(|| format!("Invalid game result string in database: {}", result))?,
            })
        } else {
            Game::Ongoing(OngoingGame {
                metadata,
                game: todo!(),
                requests: todo!(),
                events,
            })
        };

        Ok((GameId(model.id), game))
    }
    fn game_to_model(game_id: Option<GameId>, game: &Game) -> Result<game::ActiveModel, String> {
        let (metadata, result, rating_info) = match game {
            Game::Ongoing(ongoing) => (&ongoing.metadata, None, None),
            Game::Finished(finished) => (
                &finished.metadata,
                Some(game_result_to_string(&finished.result)),
                finished.rating_info.as_ref(),
            ),
        };

        let base_settings = &metadata.settings.base;
        let time_settings = JsonTimeSettings::from_time_settings(&metadata.settings.time_settings);

        let events = match game {
            Game::Ongoing(ongoing) => &ongoing.events,
            Game::Finished(finished) => &finished.events,
        };

        let events: Vec<JsonEventRecord> = events
            .iter()
            .map(|event| JsonEventRecord {
                timestamp: event.date,
                event: JsonEventRecordType::from_game_event(event.event_type.clone()),
                time_info: JsonTimeInfo::from_time_info(&event.time_info),
            })
            .collect();

        let events_value = serde_json::to_value(&events)
            .map_err(|e| format!("Failed to serialize game events: {}", e))?;

        Ok(game::ActiveModel {
            id: game_id.map_or(Default::default(), |id| Set(id.0)),
            date: Set(metadata.date),
            size: Set(base_settings.board_size as i32),
            player_white_id: Set(metadata.white.user_id.0),
            player_black_id: Set(metadata.black.user_id.0),
            player_white_username: Set(metadata.white.username.clone()),
            player_black_username: Set(metadata.black.username.clone()),
            player_white_rating: Set(metadata.white.rating),
            player_black_rating: Set(metadata.black.rating),
            events: Set(events_value),
            result: Set(result),
            is_rated: Set(metadata.is_rated),
            half_komi: Set(base_settings.half_komi as i32),
            pieces: Set(base_settings.reserve.pieces as i32),
            capstones: Set(base_settings.reserve.capstones as i32),
            opening: Set(tak_opening_to_string(&base_settings.opening)),
            rating_change_white: Set(rating_info.map(|info| info.rating_change_white)),
            rating_change_black: Set(rating_info.map(|info| info.rating_change_black)),
            game_settings: Set(serde_json::to_value(&time_settings)
                .map_err(|e| format!("Failed to serialize game settings: {}", e))?),
            match_id: Set(metadata.match_id.map(|id| id.0)),
        })
    }
}

#[async_trait::async_trait]
impl GameRepository<RepositoryContext> for GameRepositoryImpl {
    async fn get_game(
        &self,
        ctx: &RepositoryContext,
        game_id: GameId,
    ) -> Result<Option<Game>, RepoError> {
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
    async fn create_game(&self, ctx: &RepositoryContext, game: Game) -> Result<GameId, RepoError> {
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
        game: Game,
    ) -> Result<(), RepoError> {
        let model = Self::game_to_model(Some(game_id), &game)
            .map_err(|e| RepoError(format!("Failed to convert game record to model: {}", e)))?;
        game::Entity::update(model)
            .exec(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;
        Ok(())
    }
    async fn get_games_by_player(
        &self,
        ctx: &RepositoryContext,
        user_id: UserId,
    ) -> Result<Vec<(GameId, Game)>, RepoError> {
        let models = game::Entity::find()
            .filter(
                sea_orm::Condition::any()
                    .add(game::Column::PlayerWhiteId.eq(user_id.0))
                    .add(game::Column::PlayerBlackId.eq(user_id.0)),
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
    async fn get_ongoing_games(
        &self,
        ctx: &RepositoryContext,
    ) -> Result<Vec<(GameId, OngoingGame)>, RepoError> {
        let models = game::Entity::find()
            .filter(game::Column::Result.is_null())
            .all(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;

        let mut results = Vec::new();
        for model in models {
            let (game_id, game) = Self::model_to_game(model)
                .map_err(|e| RepoError(format!("Failed to convert game model to record: {}", e)))?;
            if let Game::Ongoing(ongoing_game) = game {
                results.push((game_id, ongoing_game));
            }
        }
        Ok(results)
    }
    async fn query_games(
        &self,
        ctx: &RepositoryContext,
        filter: GameQuery,
    ) -> Result<PaginatedResponse<(GameId, Game)>, RepoError> {
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

        if !filter.game_results.is_empty() {
            let mut has_empty = false;
            let result_strings: Vec<String> = filter
                .game_results
                .iter()
                .filter_map(|result| match result {
                    Some(r) => Some(game_result_to_string(r)),
                    None => {
                        has_empty = true;
                        None
                    }
                })
                .collect();
            let mut filter = game::Column::Result.is_in(result_strings);
            if has_empty {
                filter = filter.or(game::Column::Result.is_null());
            }
            query = query.filter(filter);
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
    ) -> Result<Vec<(GameId, Game)>, RepoError> {
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
