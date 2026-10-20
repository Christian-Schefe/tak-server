use std::collections::HashMap;

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use tak_core::{
    TakBaseGameSettings, TakReserve,
    ptn::{action_from_ptn, action_to_ptn},
};
use tak_domain::{
    PuzzleId,
    puzzle::{Puzzle, PuzzleResponseEntry},
};
use tak_persistence_sea_orm_entities::puzzle;
use tak_server_app::ports::{
    puzzle::PuzzleRepository,
    transaction::{TransactionError, TransactionItem},
};

use crate::{tak_opening_from_string, transaction::TransactionContext};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonResponse {
    pub responses: HashMap<String, Option<JsonResponseEntry>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonResponseEntry {
    pub action: String,
    pub response: JsonResponse,
}

impl JsonResponse {
    fn to_puzzle_response_entry(&self) -> Result<PuzzleResponseEntry, String> {
        let mut responses = HashMap::new();
        for (action_str, response) in &self.responses {
            let action = action_from_ptn(action_str)
                .ok_or_else(|| format!("Invalid action in JSON response: {}", action_str))?;
            let response_entry = match response {
                Some(json_response) => Some((
                    action_from_ptn(&json_response.action).ok_or_else(|| {
                        format!(
                            "Invalid action in JSON response entry: {}",
                            json_response.action
                        )
                    })?,
                    json_response.response.to_puzzle_response_entry()?,
                )),
                None => None,
            };
            responses.insert(action, response_entry);
        }
        Ok(PuzzleResponseEntry(responses))
    }

    #[allow(dead_code)]
    //TODO: Useful once puzzle creation is implemented, but not needed for puzzle solving
    fn from_puzzle_response_entry(entry: &PuzzleResponseEntry) -> Self {
        let responses = entry
            .0
            .iter()
            .map(|(action, response)| {
                let action_str = action_to_ptn(action);
                let response_entry =
                    response
                        .as_ref()
                        .map(|(resp_action, resp_entry)| JsonResponseEntry {
                            action: action_to_ptn(resp_action),
                            response: Self::from_puzzle_response_entry(resp_entry),
                        });
                (action_str, response_entry)
            })
            .collect();
        JsonResponse { responses }
    }
}

pub struct PuzzleRepositoryImpl;

impl PuzzleRepositoryImpl {
    fn puzzle_from_entity(model: puzzle::Model) -> Result<Puzzle, String> {
        let base_settings = TakBaseGameSettings {
            board_size: model.size as u32,
            half_komi: model.half_komi as u32,
            reserve: TakReserve::new(model.pieces as u32, model.capstones as u32),
            opening: tak_opening_from_string(&model.opening)
                .ok_or_else(|| format!("Invalid opening in puzzle: {}", model.opening))?,
        };
        let position = serde_json::from_value::<Vec<String>>(model.position)
            .map_err(|e| format!("Invalid Position: {}", e.to_string()))?
            .into_iter()
            .map(|s| action_from_ptn(&s).ok_or_else(|| format!("Invalid Position: {}", s)))
            .collect::<Result<Vec<_>, String>>()?;
        let response = serde_json::from_value::<JsonResponse>(model.responses)
            .map_err(|e| format!("Invalid Responses: {}", e.to_string()))?;
        let response = response
            .to_puzzle_response_entry()
            .map_err(|e| format!("Invalid Responses: {}", e))?;

        Ok(Puzzle {
            id: PuzzleId(model.id),
            game_settings: base_settings,
            position,
            responses: response,
        })
    }
}

impl PuzzleRepository<TransactionContext> for PuzzleRepositoryImpl {
    fn get_puzzle(
        &self,
        id: PuzzleId,
    ) -> TransactionItem<TransactionContext, Result<Option<Puzzle>, TransactionError>> {
        TransactionItem::simple(Box::new(move |ctx| {
            Box::pin(async move {
                let puzzle_entity = puzzle::Entity::find_by_id(id.0)
                    .one(ctx)
                    .await
                    .map_err(|e| TransactionError(e.to_string()))?;
                if let Some(puzzle_entity) = puzzle_entity {
                    let puzzle = Self::puzzle_from_entity(puzzle_entity).map_err(|e| {
                        TransactionError(format!("Failed to convert puzzle entity: {:?}", e))
                    })?;
                    Ok(Some(puzzle))
                } else {
                    Ok(None)
                }
            })
        }))
    }

    fn select_random_puzzle(
        &self,
    ) -> TransactionItem<TransactionContext, Result<PuzzleId, TransactionError>> {
        const SAMPLE_SIZE: u64 = 20;

        TransactionItem::simple(Box::new(move |ctx| {
            Box::pin(async move {
                let random_seed: f64 = rand::random_range(0.0..1.0);
                let puzzle_entity = puzzle::Entity::find()
                    .filter(puzzle::Column::RandomSeed.gt(random_seed))
                    .limit(SAMPLE_SIZE)
                    .order_by_asc(puzzle::Column::RandomSeed)
                    .all(ctx)
                    .await
                    .map_err(|e| TransactionError(e.to_string()))?;
                if !puzzle_entity.is_empty() {
                    let index = rand::random_range(0..puzzle_entity.len());
                    return Ok(PuzzleId(puzzle_entity[index].id));
                }
                let puzzle_entity = puzzle::Entity::find()
                    .limit(SAMPLE_SIZE)
                    .order_by_asc(puzzle::Column::RandomSeed)
                    .all(ctx)
                    .await
                    .map_err(|e| TransactionError(e.to_string()))?;
                if !puzzle_entity.is_empty() {
                    let index = rand::random_range(0..puzzle_entity.len());
                    return Ok(PuzzleId(puzzle_entity[index].id));
                }
                Err(TransactionError(
                    "Failed to select a random puzzle id".to_string(),
                ))
            })
        }))
    }
}
