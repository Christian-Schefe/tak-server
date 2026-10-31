use std::time::Duration;

use sea_orm::{ActiveValue::Set, EntityTrait, Iterable, sea_query::OnConflict};
use tak_core::{
    TakActionRecord, TakClock,
    ptn::{action_from_ptn, action_to_ptn, game_result_from_string, game_result_to_string},
};
use tak_domain::{
    GameId,
    game::{FinishedGameState, GameRequests, GameState, OngoingGameState},
};
use tak_persistence_sea_orm_entities::game_state;
use tak_server_app::ports::{game::GameStateRepository, transaction::RepoError};

use crate::{
    games::{JsonActionRecord, JsonTimeInfo},
    transaction::RepositoryContext,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct JsonClock {
    remaining_time: JsonTimeInfo,
    is_ticking: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct JsonGameRequests {
    draw_offered: bool,
    undo_requested: bool,
    more_time_offered: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct JsonOngoingGameState {
    clock: JsonClock,
    white_requests: JsonGameRequests,
    black_requests: JsonGameRequests,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct JsonFinishedGameState {
    result: String,
    time_info: JsonTimeInfo,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum JsonGameState {
    Ongoing(JsonOngoingGameState),
    Finished(JsonFinishedGameState),
}

pub struct GameStateRepositoryImpl;

impl GameStateRepositoryImpl {
    fn model_to_game(model: game_state::Model) -> Result<GameState, String> {
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

        let state = serde_json::from_value::<JsonGameState>(model.state)
            .map_err(|e| format!("Failed to deserialize game state from JSON: {}", e))?;
        match state {
            JsonGameState::Ongoing(state) => Ok(GameState::Ongoing(OngoingGameState {
                history,
                clock: TakClock {
                    remaining_time: state.clock.remaining_time.to_time_info(),
                    is_ticking: state.clock.is_ticking.map(|t| t.into()),
                },
                white_requests: GameRequests {
                    draw_offered: state.white_requests.draw_offered,
                    undo_requested: state.white_requests.undo_requested,
                    more_time_offered: state
                        .white_requests
                        .more_time_offered
                        .map(|ms| Duration::from_millis(ms)),
                },
                black_requests: GameRequests {
                    draw_offered: state.black_requests.draw_offered,
                    undo_requested: state.black_requests.undo_requested,
                    more_time_offered: state
                        .black_requests
                        .more_time_offered
                        .map(|ms| Duration::from_millis(ms)),
                },
            })),
            JsonGameState::Finished(json_finished_game_state) => {
                let result = game_result_from_string(&json_finished_game_state.result)
                    .ok_or_else(|| format!("Failed to parse game result from string"))?;
                Ok(GameState::Finished(FinishedGameState {
                    history,
                    time_info: json_finished_game_state.time_info.to_time_info(),
                    result,
                }))
            }
        }
    }

    fn game_to_model(
        game_id: GameId,
        state: &GameState,
    ) -> Result<game_state::ActiveModel, String> {
        let history = match state {
            GameState::Ongoing(ongoing_state) => &ongoing_state.history,
            GameState::Finished(finished_state) => &finished_state.history,
        };
        let history: Vec<JsonActionRecord> = history
            .iter()
            .map(|action_record| JsonActionRecord {
                action: action_to_ptn(&action_record.action),
                time_info: JsonTimeInfo::from_time_info(&action_record.time_info),
            })
            .collect();

        let state_json = match state {
            GameState::Ongoing(ongoing_state) => {
                let json_state = JsonOngoingGameState {
                    clock: JsonClock {
                        remaining_time: JsonTimeInfo::from_time_info(
                            &ongoing_state.clock.remaining_time,
                        ),
                        is_ticking: ongoing_state.clock.is_ticking.map(|t| t.into()),
                    },
                    white_requests: JsonGameRequests {
                        draw_offered: ongoing_state.white_requests.draw_offered,
                        undo_requested: ongoing_state.white_requests.undo_requested,
                        more_time_offered: ongoing_state
                            .white_requests
                            .more_time_offered
                            .map(|d| d.as_millis() as u64),
                    },
                    black_requests: JsonGameRequests {
                        draw_offered: ongoing_state.black_requests.draw_offered,
                        undo_requested: ongoing_state.black_requests.undo_requested,
                        more_time_offered: ongoing_state
                            .black_requests
                            .more_time_offered
                            .map(|d| d.as_millis() as u64),
                    },
                };
                serde_json::to_value(&JsonGameState::Ongoing(json_state))
            }
            GameState::Finished(finished_state) => {
                let json_state = JsonFinishedGameState {
                    result: game_result_to_string(&finished_state.result),
                    time_info: JsonTimeInfo::from_time_info(&finished_state.time_info),
                };
                serde_json::to_value(&JsonGameState::Finished(json_state))
            }
        };
        Ok(game_state::ActiveModel {
            id: Set(game_id.0),
            history: Set(serde_json::to_value(&history)
                .map_err(|e| format!("Failed to serialize game history to JSON: {}", e))?),
            state: Set(
                state_json.map_err(|e| format!("Failed to serialize game state to JSON: {}", e))?
            ),
        })
    }
}

#[async_trait::async_trait]
impl GameStateRepository<RepositoryContext> for GameStateRepositoryImpl {
    async fn set_state(
        &self,
        ctx: &RepositoryContext,
        game_id: GameId,
        game: GameState,
    ) -> Result<(), RepoError> {
        let active_model = Self::game_to_model(game_id, &game)
            .map_err(|e| RepoError(format!("Failed to convert game state to model: {}", e)))?;
        game_state::Entity::insert(active_model)
            .on_conflict(
                OnConflict::column(game_state::Column::Id)
                    .update_columns(
                        game_state::Column::iter()
                            .filter(|c| !matches!(c, game_state::Column::Id))
                            .collect::<Vec<_>>(),
                    )
                    .to_owned(),
            )
            .exec(ctx)
            .await
            .map_err(|e| RepoError(format!("Failed to insert/update game state: {}", e)))?;
        Ok(())
    }
    async fn get_state(
        &self,
        ctx: &RepositoryContext,
        game_id: GameId,
    ) -> Result<Option<GameState>, RepoError> {
        let model = game_state::Entity::find_by_id(game_id.0)
            .one(ctx)
            .await
            .map_err(|e| RepoError(format!("Failed to retrieve game state: {}", e)))?;
        match model {
            Some(model) => {
                let game_state = Self::model_to_game(model).map_err(|e| {
                    RepoError(format!("Failed to convert model to game state: {}", e))
                })?;
                Ok(Some(game_state))
            }
            None => Ok(None),
        }
    }
}
