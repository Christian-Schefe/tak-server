use std::time::Duration;

use chrono::DateTime;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use tak_core::{
    TakAction, TakActionRecord, TakDir, TakGameSettings, TakGameState, TakPos, TakTimeControl,
    TakVariant,
    ptn::{game_state_from_string, game_state_to_string},
};
use tak_server_domain::{
    ServiceError, ServiceResult,
    game::{GameId, GameRatingUpdate, GameRecord, GameRepository, GameResultUpdate, GameType},
    player::Player,
    rating::GameRatingInfo,
};

use crate::{create_games_db_pool, entity::game};

pub struct GameRepositoryImpl {
    db: DatabaseConnection,
}

impl GameRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_games_db_pool().await;
        Self { db }
    }

    fn action_record_to_database_string(record: &TakActionRecord) -> String {
        fn square_to_string(pos: &TakPos) -> String {
            format!(
                "{}{}",
                (b'A' + pos.x as u8) as char,
                (b'1' + pos.y as u8) as char,
            )
        }
        match &record.action {
            TakAction::Place { pos, variant } => format!(
                "P {} {}",
                square_to_string(pos),
                match variant {
                    TakVariant::Flat => "",
                    TakVariant::Standing => "S",
                    TakVariant::Capstone => "C",
                },
            ),
            TakAction::Move { pos, dir, drops } => {
                let to_pos = pos.offset(dir, drops.len() as i32);
                let drops_str = drops
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join("");
                format!(
                    "M {} {} {}",
                    square_to_string(pos),
                    square_to_string(&to_pos),
                    drops_str
                )
            }
        }
    }

    fn action_record_from_database_string(s: &str) -> Option<TakActionRecord> {
        fn square_from_string(s: &str) -> Option<TakPos> {
            if s.len() != 2 {
                return None;
            }
            let x = s.chars().nth(0)?;
            let y = s.chars().nth(1)?;
            if !('A'..='Z').contains(&x) || !('1'..='9').contains(&y) {
                return None;
            }
            Some(TakPos {
                x: (x as u8 - b'A') as i32,
                y: (y as u8 - b'1') as i32,
            })
        }
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }
        match parts[0] {
            "P" if parts.len() == 3 => {
                let pos_str = parts[1];
                let variant_str = parts[2];
                let pos = square_from_string(&pos_str)?;
                let variant = match variant_str {
                    "" => TakVariant::Flat,
                    "S" => TakVariant::Standing,
                    "C" => TakVariant::Capstone,
                    _ => return None,
                };
                Some(TakActionRecord {
                    action: TakAction::Place { pos, variant },
                    time_remaining: (Duration::ZERO, Duration::ZERO),
                })
            }
            "M" if parts.len() == 4 => {
                let from_str = parts[1];
                let to_str = parts[2];
                let drops_str = parts[3];
                let from_pos = square_from_string(&from_str)?;
                let to_pos = square_from_string(&to_str)?;
                let dir = if to_pos.x > from_pos.x {
                    TakDir::Right
                } else if to_pos.x < from_pos.x {
                    TakDir::Left
                } else if to_pos.y > from_pos.y {
                    TakDir::Up
                } else {
                    TakDir::Down
                };
                let drops = drops_str
                    .chars()
                    .filter_map(|c| c.to_digit(10).map(|d| d as u32))
                    .collect::<Vec<_>>();
                Some(TakActionRecord {
                    action: TakAction::Move {
                        pos: from_pos,
                        dir,
                        drops,
                    },
                    time_remaining: (Duration::ZERO, Duration::ZERO),
                })
            }
            _ => None,
        }
    }

    fn model_to_game(model: &game::Model) -> GameRecord {
        let rating_info = if model.rating_change_white != 1000 || model.rating_change_black != 1000
        {
            Some(GameRatingInfo {
                rating_white: model.rating_white as f64,
                rating_black: model.rating_black as f64,
                rating_change_white: model.rating_change_white as f64,
                rating_change_black: model.rating_change_black as f64,
            })
        } else {
            None
        };
        let time_control = TakTimeControl {
            contingent: Duration::from_secs(model.timertime as u64),
            increment: Duration::from_secs(model.timerinc as u64),
            extra: if model.extra_time_amount > 0 && model.extra_time_trigger > 0 {
                Some((
                    model.extra_time_amount as u32,
                    Duration::from_secs(model.extra_time_trigger as u64),
                ))
            } else {
                None
            },
        };
        GameRecord {
            date: DateTime::from_timestamp_secs(model.date).unwrap_or_default(),
            settings: TakGameSettings {
                board_size: model.size as u32,
                time_control,
                half_komi: model.komi as u32,
                reserve_pieces: model.pieces as u32,
                reserve_capstones: model.capstones as u32,
            },
            white: model.player_white.clone(),
            black: model.player_black.clone(),
            game_type: if model.unrated {
                GameType::Unrated
            } else if model.tournament {
                GameType::Tournament
            } else {
                GameType::Rated
            },
            moves: model
                .notation
                .split(',')
                .filter_map(|s| Self::action_record_from_database_string(s))
                .collect(),
            rating_info,
            result: game_state_from_string(&model.result).unwrap_or(TakGameState::Ongoing),
        }
    }
}

#[async_trait::async_trait]
impl GameRepository for GameRepositoryImpl {
    async fn create_game(
        &self,
        game: &GameRecord,
        player_white: &Player,
        player_black: &Player,
    ) -> ServiceResult<GameId> {
        let new_game = game::ActiveModel {
            id: Default::default(), // Auto-increment
            date: Set(game.date.timestamp()),
            size: Set(game.settings.board_size as i32),
            player_white: Set(game.white.clone()),
            player_black: Set(game.black.clone()),
            notation: Set(String::new()),
            result: Set("0-0".to_string()),
            timertime: Set(game.settings.time_control.contingent.as_secs() as i32),
            timerinc: Set(game.settings.time_control.increment.as_secs() as i32),
            rating_white: Set(player_white.rating.rating as i32),
            rating_black: Set(player_black.rating.rating as i32),
            unrated: Set(game.game_type == GameType::Unrated),
            tournament: Set(game.game_type == GameType::Tournament),
            komi: Set(game.settings.half_komi as i32),
            pieces: Set(game.settings.reserve_pieces as i32),
            capstones: Set(game.settings.reserve_capstones as i32),
            rating_change_white: Set(-1000),
            rating_change_black: Set(-1000),
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
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(result.id)
    }

    async fn update_game_result(&self, id: GameId, update: &GameResultUpdate) -> ServiceResult<()> {
        let notation_val = update
            .moves
            .iter()
            .map(|action| Self::action_record_to_database_string(action))
            .collect::<Vec<_>>()
            .join(",");
        let result_val = game_state_to_string(&update.result);

        let game = game::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?
            .ok_or_else(|| ServiceError::Internal("Game not found".to_string()))?;

        let mut game: game::ActiveModel = game.into();
        game.notation = Set(notation_val);
        game.result = Set(result_val);

        game.update(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn update_game_rating(&self, id: GameId, update: &GameRatingUpdate) -> ServiceResult<()> {
        let game = game::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?
            .ok_or_else(|| ServiceError::Internal("Game not found".to_string()))?;

        let mut game: game::ActiveModel = game.into();
        game.rating_white = Set(update.rating_info.rating_white as i32);
        game.rating_black = Set(update.rating_info.rating_black as i32);
        game.rating_change_white = Set(update.rating_info.rating_change_white as i32);
        game.rating_change_black = Set(update.rating_info.rating_change_black as i32);

        game.update(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn get_games(&self) -> ServiceResult<Vec<(GameId, GameRecord)>> {
        let models = game::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        let mut results = Vec::new();
        for model in models {
            let game_record = Self::model_to_game(&model);
            results.push((model.id, game_record));
        }

        Ok(results)
    }
}
