use std::collections::HashMap;

use sea_orm::{
    ActiveValue::Set,
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, Iterable, QueryFilter,
    QueryOrder, QuerySelect,
    sea_query::{Func, OnConflict, Query},
};
use tak_core::{
    TakBaseGameSettings, TakOpening, TakReserve,
    ptn::{action_from_ptn, action_to_ptn},
};
use tak_persistence_sea_orm_entities::puzzle;
use tak_server_app::domain::{
    PuzzleId, RepoError, RepoRetrieveError,
    puzzle::{Puzzle, PuzzleRepository, PuzzleResponseEntry},
};

use crate::{create_db_pool, tak_opening_from_string, tak_opening_to_string};

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

pub struct PuzzleRepositoryImpl {
    db: DatabaseConnection,
}

impl PuzzleRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;

        let this = Self { db };
        this.save_puzzle(&test_puzzle())
            .await
            .expect("Failed to save test puzzle");
        this.save_puzzle(&test_puzzle2())
            .await
            .expect("Failed to save test puzzle 2");
        this.reshuffle_puzzles()
            .await
            .expect("Failed to reshuffle puzzles");
        this
    }

    fn puzzle_from_entity(&self, model: puzzle::Model) -> Result<Puzzle, RepoRetrieveError> {
        let base_settings = TakBaseGameSettings {
            board_size: model.size as u32,
            half_komi: model.half_komi as u32,
            reserve: TakReserve::new(model.pieces as u32, model.capstones as u32),
            opening: tak_opening_from_string(&model.opening).ok_or_else(|| {
                RepoRetrieveError::StorageError(format!(
                    "Invalid opening in puzzle: {}",
                    model.opening
                ))
            })?,
        };
        let position = serde_json::from_value::<Vec<String>>(model.position)
            .map_err(|e| {
                RepoRetrieveError::StorageError(format!("Invalid Position: {}", e.to_string()))
            })?
            .into_iter()
            .map(|s| {
                action_from_ptn(&s).ok_or_else(|| {
                    RepoRetrieveError::StorageError(format!("Invalid Position: {}", s))
                })
            })
            .collect::<Result<Vec<_>, RepoRetrieveError>>()?;
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

    async fn reshuffle_puzzles(&self) -> Result<(), RepoError> {
        let query = Query::update()
            .table(puzzle::Entity)
            .value(puzzle::Column::RandomSeed, Func::random())
            .to_owned();
        self.db.execute(&query).await.map_err(|e| {
            RepoError::StorageError(format!("Failed to reshuffle puzzles: {}", e.to_string()))
        })?;
        Ok(())
    }

    async fn save_puzzle(&self, puzzle: &Puzzle) -> Result<(), RepoError> {
        let responses =
            serde_json::to_value(&JsonResponse::from_puzzle_response_entry(&puzzle.responses))
                .map_err(|e| {
                    RepoError::StorageError(format!(
                        "Failed to serialize puzzle responses: {}",
                        e.to_string()
                    ))
                })?;

        let position_val = serde_json::to_value(
            &puzzle
                .position
                .iter()
                .map(|a| action_to_ptn(a))
                .collect::<Vec<_>>(),
        )
        .map_err(|e| {
            RepoError::StorageError(format!(
                "Failed to serialize puzzle position: {}",
                e.to_string()
            ))
        })?;

        let model = puzzle::ActiveModel {
            id: Set(puzzle.id.0),
            size: Set(puzzle.game_settings.board_size as i32),
            half_komi: Set(puzzle.game_settings.half_komi as i32),
            pieces: Set(puzzle.game_settings.reserve.pieces as i32),
            capstones: Set(puzzle.game_settings.reserve.capstones as i32),
            opening: Set(tak_opening_to_string(&puzzle.game_settings.opening)),
            position: Set(position_val),
            responses: Set(responses),
            random_seed: Set(rand::random_range(0.0..1.0)),
        };
        puzzle::Entity::insert(model)
            .on_conflict(
                OnConflict::column(puzzle::Column::Id)
                    .update_columns(
                        puzzle::Column::iter()
                            .filter(|c| !matches!(c, puzzle::Column::Id))
                            .collect::<Vec<_>>(),
                    )
                    .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(|e| {
                RepoError::StorageError(format!(
                    "Failed to save puzzle to database: {}",
                    e.to_string()
                ))
            })?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl PuzzleRepository for PuzzleRepositoryImpl {
    async fn get_puzzle(&self, id: PuzzleId) -> Result<Puzzle, RepoRetrieveError> {
        let puzzle_entity = puzzle::Entity::find_by_id(id.0)
            .one(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?
            .ok_or(RepoRetrieveError::NotFound)?;
        self.puzzle_from_entity(puzzle_entity)
    }

    async fn select_random_puzzle(&self) -> Result<PuzzleId, RepoError> {
        const SAMPLE_SIZE: u64 = 20;

        let random_seed: f64 = rand::random_range(0.0..1.0);
        let puzzle_entity = puzzle::Entity::find()
            .filter(puzzle::Column::RandomSeed.gt(random_seed))
            .limit(SAMPLE_SIZE)
            .order_by_asc(puzzle::Column::RandomSeed)
            .all(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;
        if !puzzle_entity.is_empty() {
            let index = rand::random_range(0..puzzle_entity.len());
            return Ok(PuzzleId(puzzle_entity[index].id));
        }
        let puzzle_entity = puzzle::Entity::find()
            .limit(SAMPLE_SIZE)
            .order_by_asc(puzzle::Column::RandomSeed)
            .all(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;
        if !puzzle_entity.is_empty() {
            let index = rand::random_range(0..puzzle_entity.len());
            return Ok(PuzzleId(puzzle_entity[index].id));
        }
        Err(RepoError::StorageError(
            "Failed to select a random puzzle id".to_string(),
        ))
    }
}

fn test_puzzle() -> Puzzle {
    let base_settings = TakBaseGameSettings {
        board_size: 5,
        half_komi: 2,
        reserve: TakReserve::from_size(5).unwrap(),
        opening: TakOpening::Swap,
    };
    let position = vec![
        action_from_ptn("a5").unwrap(),
        action_from_ptn("e5").unwrap(),
        action_from_ptn("e4").unwrap(),
    ];
    let responses = PuzzleResponseEntry(HashMap::from([((
        action_from_ptn("a4").unwrap(),
        Some((
            action_from_ptn("e3").unwrap(),
            PuzzleResponseEntry(HashMap::from([(action_from_ptn("a3").unwrap(), None)])),
        )),
    ))]));
    Puzzle {
        id: PuzzleId(0),
        game_settings: base_settings,
        position,
        responses,
    }
}

fn test_puzzle2() -> Puzzle {
    let base_settings = TakBaseGameSettings {
        board_size: 6,
        half_komi: 2,
        reserve: TakReserve::from_size(6).unwrap(),
        opening: TakOpening::Swap,
    };
    let position = vec![
        action_from_ptn("a5").unwrap(),
        action_from_ptn("e5").unwrap(),
        action_from_ptn("Ca4").unwrap(),
    ];
    let responses = PuzzleResponseEntry(HashMap::from([((
        action_from_ptn("Cb4").unwrap(),
        Some((
            action_from_ptn("a4-").unwrap(),
            PuzzleResponseEntry(HashMap::from([(action_from_ptn("b4-").unwrap(), None)])),
        )),
    ))]));
    Puzzle {
        id: PuzzleId(1),
        game_settings: base_settings,
        position,
        responses,
    }
}
