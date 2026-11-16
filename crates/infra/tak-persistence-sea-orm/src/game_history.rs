use sea_orm::{
    ActiveValue::{NotSet, Set},
    ColumnTrait, EntityTrait, QueryFilter,
};
use tak_domain::{GameId, game::GameEvent};
use tak_persistence_sea_orm_entities::game_history;
use tak_server_app::ports::{game::GameHistoryRepository, transaction::RepoError};

use crate::{game_event_from_json, game_event_to_json, transaction::RepositoryContext};

pub struct GameHistoryRepositoryImpl;

impl GameHistoryRepositoryImpl {
    fn model_to_event(model: game_history::Model) -> Result<GameEvent, String> {
        game_event_from_json(&model.event)
    }

    fn event_to_model(game_id: GameId, event: &GameEvent) -> game_history::ActiveModel {
        game_history::ActiveModel {
            id: NotSet,
            event: Set(game_event_to_json(event)),
            game_id: Set(game_id.0),
        }
    }
}

#[async_trait::async_trait]
impl GameHistoryRepository<RepositoryContext> for GameHistoryRepositoryImpl {
    async fn add_events(
        &self,
        ctx: &RepositoryContext,
        game_id: GameId,
        events: Vec<GameEvent>,
    ) -> Result<(), RepoError> {
        let models: Vec<_> = events
            .into_iter()
            .map(|event| Self::event_to_model(game_id, &event))
            .collect();
        game_history::Entity::insert_many(models)
            .exec(ctx)
            .await
            .map_err(|e| RepoError(format!("Failed to insert game event: {}", e)))?;

        Ok(())
    }

    async fn get_events(
        &self,
        ctx: &RepositoryContext,
        game_id: GameId,
    ) -> Result<Vec<GameEvent>, RepoError> {
        let models = game_history::Entity::find()
            .filter(game_history::Column::GameId.eq(game_id.0))
            .order_by_id_asc()
            .all(ctx)
            .await
            .map_err(|e| RepoError(format!("Failed to fetch game events: {}", e)))?;
        models
            .into_iter()
            .map(|model| {
                Self::model_to_event(model)
                    .map_err(|e| RepoError(format!("Failed to convert model to event: {}", e)))
            })
            .collect::<Result<Vec<_>, _>>()
    }
}
