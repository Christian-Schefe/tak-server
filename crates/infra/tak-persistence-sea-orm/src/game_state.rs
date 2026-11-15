use std::time::Duration;

use sea_orm::{ActiveValue::Set, EntityTrait, Iterable, sea_query::OnConflict};
use tak_core::{
    TakActionRecord, TakClock,
    ptn::{action_from_ptn, action_to_ptn},
};
use tak_domain::{
    GameId,
    game::{GameRequests, GameState},
};
use tak_persistence_sea_orm_entities::game_state;
use tak_server_app::ports::{game::GameStateRepository, transaction::RepoError};

use crate::{
    games::{JsonActionRecord, JsonTimeInfo},
    transaction::RepositoryContext,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonClock {
    remaining_time: JsonTimeInfo,
    is_ticking: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonGameRequests {
    draw_offered: bool,
    undo_requested: bool,
    more_time_offered: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonGameState {
    history: Vec<JsonActionRecord>,
    clock: JsonClock,
    white_requests: JsonGameRequests,
    black_requests: JsonGameRequests,
}

pub struct GameStateRepositoryImpl;

impl GameStateRepositoryImpl {
    fn model_to_game(model: game_state::Model) -> Result<GameState, String> {
        let state = serde_json::from_value::<JsonGameState>(model.state)
            .map_err(|e| format!("Failed to deserialize game state from JSON: {}", e))?;
        Ok(GameState {
            history: state
                .history
                .into_iter()
                .map(|json_action_record: JsonActionRecord| {
                    Ok(TakActionRecord {
                        action: action_from_ptn(&json_action_record.action)
                            .ok_or_else(|| format!("Failed to parse action from PTN"))?,
                        time_info: json_action_record.time_info.to_time_info(),
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
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
        })
    }

    fn game_to_model(
        game_id: GameId,
        state: &GameState,
    ) -> Result<game_state::ActiveModel, String> {
        let history: Vec<JsonActionRecord> = state
            .history
            .iter()
            .map(|action_record| JsonActionRecord {
                action: action_to_ptn(&action_record.action),
                time_info: JsonTimeInfo::from_time_info(&action_record.time_info),
            })
            .collect();

        let state_json = serde_json::to_value(&JsonGameState {
            history,
            clock: JsonClock {
                remaining_time: JsonTimeInfo::from_time_info(&state.clock.remaining_time),
                is_ticking: state.clock.is_ticking.map(|t| t.into()),
            },
            white_requests: JsonGameRequests {
                draw_offered: state.white_requests.draw_offered,
                undo_requested: state.white_requests.undo_requested,
                more_time_offered: state
                    .white_requests
                    .more_time_offered
                    .map(|d| d.as_millis() as u64),
            },
            black_requests: JsonGameRequests {
                draw_offered: state.black_requests.draw_offered,
                undo_requested: state.black_requests.undo_requested,
                more_time_offered: state
                    .black_requests
                    .more_time_offered
                    .map(|d| d.as_millis() as u64),
            },
        });
        Ok(game_state::ActiveModel {
            id: Set(game_id.0),
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
    async fn remove_state(
        &self,
        ctx: &RepositoryContext,
        game_id: GameId,
    ) -> Result<(), RepoError> {
        game_state::Entity::delete_by_id(game_id.0)
            .exec(ctx)
            .await
            .map_err(|e| RepoError(format!("Failed to delete game state: {}", e)))?;
        Ok(())
    }
}
