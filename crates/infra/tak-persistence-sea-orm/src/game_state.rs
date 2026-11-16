use sea_orm::{ActiveValue::Set, EntityTrait, Iterable, sea_query::OnConflict};
use tak_domain::{GameId, game::GameState};
use tak_persistence_sea_orm_entities::game_state;
use tak_server_app::ports::{game::GameStateRepository, transaction::RepoError};

use crate::{
    game_requests_from_json, game_requests_to_json, history_from_json, history_to_json,
    tak_clock_from_json, tak_clock_to_json, transaction::RepositoryContext,
};

pub struct GameStateRepositoryImpl;

impl GameStateRepositoryImpl {
    fn model_to_game(model: game_state::Model) -> Result<GameState, String> {
        Ok(GameState {
            history: history_from_json(&model.history)?,
            clock: tak_clock_from_json(&model.clock),
            white_requests: game_requests_from_json(&model.white_requests),
            black_requests: game_requests_from_json(&model.black_requests),
        })
    }

    fn game_to_model(game_id: GameId, state: &GameState) -> game_state::ActiveModel {
        game_state::ActiveModel {
            id: Set(game_id.0),
            history: Set(history_to_json(&state.history)),
            clock: Set(tak_clock_to_json(&state.clock)),
            white_requests: Set(game_requests_to_json(&state.white_requests)),
            black_requests: Set(game_requests_to_json(&state.black_requests)),
        }
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
        let active_model = Self::game_to_model(game_id, &game);
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
