use std::collections::HashMap;

use sea_orm::{DatabaseConnection, EntityTrait};
use tak_core::{
    TakBaseGameSettings, TakReserve,
    ptn::{action_from_ptn, game_position_from_string},
};
use tak_persistence_sea_orm_entities::puzzle;
use tak_server_app::domain::{
    PuzzleId, RepoRetrieveError,
    puzzle::{Puzzle, PuzzleRepository, PuzzleResponseEntry},
};

use crate::create_db_pool;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonResponse {
    pub action: String,
    pub responses: HashMap<String, Option<JsonResponse>>,
}

impl JsonResponse {
    fn to_puzzle_response_entry(&self) -> Result<PuzzleResponseEntry, String> {
        let action = action_from_ptn(&self.action)
            .ok_or_else(|| format!("Invalid action in JSON response: {}", self.action))?;
        let mut responses = HashMap::new();
        for (action_str, response) in &self.responses {
            let action = action_from_ptn(action_str)
                .ok_or_else(|| format!("Invalid action in JSON response: {}", action_str))?;
            let response_entry = match response {
                Some(json_response) => Some(json_response.to_puzzle_response_entry()?),
                None => None,
            };
            responses.insert(action, response_entry);
        }
        Ok(PuzzleResponseEntry::new(action, responses))
    }
}

pub struct PuzzleRepositoryImpl {
    db: DatabaseConnection,
}

impl PuzzleRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
        Self { db }
    }

    fn puzzle_from_entity(&self, model: puzzle::Model) -> Result<Puzzle, RepoRetrieveError> {
        let base_settings = TakBaseGameSettings {
            board_size: model.size as u32,
            half_komi: model.half_komi as u32,
            reserve: TakReserve::new(model.pieces as u32, model.capstones as u32),
        };
        let position = game_position_from_string(&model.position).ok_or(
            RepoRetrieveError::StorageError("Invalid Position".to_string()),
        )?;
        let response = serde_json::from_value::<JsonResponse>(model.responses).map_err(|e| {
            RepoRetrieveError::StorageError(format!("Invalid Responses: {}", e.to_string()))
        })?;
        let response = response
            .to_puzzle_response_entry()
            .map_err(|e| RepoRetrieveError::StorageError(format!("Invalid Responses: {}", e)))?;

        Ok(Puzzle {
            id: PuzzleId(model.id),
            game_settings: base_settings,
            position,
            responses: response,
        })
    }
}

#[async_trait::async_trait]
impl PuzzleRepository for PuzzleRepositoryImpl {
    async fn get_puzzle(&self, id: PuzzleId) -> Result<Puzzle, RepoRetrieveError> {
        if id.0 == 0 {
            // Temporary hardcoded puzzle for testing
            return Ok(test_puzzle());
        }
        let puzzle_entity = puzzle::Entity::find_by_id(id.0)
            .one(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?
            .ok_or(RepoRetrieveError::NotFound)?;
        self.puzzle_from_entity(puzzle_entity)
    }
}

fn test_puzzle() -> Puzzle {
    let base_settings = TakBaseGameSettings {
        board_size: 5,
        half_komi: 2,
        reserve: TakReserve::new(21, 1),
    };
    let position = game_position_from_string("1,x3,2/x5/x5/x5/x5 1 2").unwrap();
    let responses = PuzzleResponseEntry::new(
        action_from_ptn("e4").unwrap(),
        HashMap::from([
            (
                action_from_ptn("a4").unwrap(),
                Some(PuzzleResponseEntry::new(
                    action_from_ptn("e3").unwrap(),
                    HashMap::from([(action_from_ptn("a3").unwrap(), None)]),
                )),
            ),
            (
                action_from_ptn("b5").unwrap(),
                Some(PuzzleResponseEntry::new(
                    action_from_ptn("e1").unwrap(),
                    HashMap::from([(action_from_ptn("Cc5").unwrap(), None)]),
                )),
            ),
        ]),
    );
    Puzzle {
        id: PuzzleId(0),
        game_settings: base_settings,
        position,
        responses,
    }
}
