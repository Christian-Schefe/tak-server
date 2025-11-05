use sqlx::{Pool, Sqlite};
use tak_core::{TakAction, TakPos, TakVariant, ptn::game_state_to_string};
use tak_server_domain::{
    ServiceError, ServiceResult,
    game::{GameId, GameRecord, GameRecordUpdate, GameRepository, GameType},
};

use crate::sqlite::create_games_db_pool;

#[derive(Debug)]
pub struct GameEntity {
    pub date: i64,
    pub size: i32,
    pub player_white: String,
    pub player_black: String,
    pub notation: String,
    pub result: String,
    pub timertime: i32,
    pub timerinc: i32,
    pub rating_white: i32,
    pub rating_black: i32,
    pub unrated: bool,
    pub tournament: bool,
    pub komi: i32,
    pub pieces: i32,
    pub capstones: i32,
    pub rating_change_white: i32,
    pub rating_change_black: i32,
    pub extra_time_amount: i32,
    pub extra_time_trigger: i32,
}

pub struct SqliteGameRepository {
    pool: Pool<Sqlite>,
}

impl SqliteGameRepository {
    pub fn new() -> Self {
        let pool = create_games_db_pool();
        Self { pool }
    }

    async fn create_game(&self, game: &GameEntity) -> ServiceResult<GameId> {
        // Id is auto-incremented
        let res = sqlx::query(
            "INSERT INTO games (date, size, player_white, player_black, notation, result, timertime, timerinc, rating_white, rating_black, unrated, tournament, komi, pieces, capstones, rating_change_white, rating_change_black, extra_time_amount, extra_time_trigger) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(game.date)
        .bind(game.size)
        .bind(&game.player_white)
        .bind(&game.player_black)
        .bind(&game.notation)
        .bind(&game.result)
        .bind(game.timertime)
        .bind(game.timerinc)
        .bind(game.rating_white)
        .bind(game.rating_black)
        .bind(if game.unrated { 1 } else { 0 })
        .bind(if game.tournament { 1 } else { 0 })
        .bind(game.komi)
        .bind(game.pieces)
        .bind(game.capstones)
        .bind(game.rating_change_white)
        .bind(game.rating_change_black)
        .bind(game.extra_time_amount)
        .bind(game.extra_time_trigger)
        .execute(&self.pool).await
        .map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(res.last_insert_rowid())
    }

    fn move_to_database_string(action: &TakAction) -> String {
        fn square_to_string(pos: &TakPos) -> String {
            format!(
                "{}{}",
                (b'A' + pos.x as u8) as char,
                (b'1' + pos.y as u8) as char,
            )
        }
        match action {
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
}

#[async_trait::async_trait]
impl GameRepository for SqliteGameRepository {
    async fn create_game(&self, game: &GameRecord) -> ServiceResult<GameId> {
        let game_entity = GameEntity {
            date: game.date.timestamp(),
            size: game.settings.board_size as i32,
            player_white: game.white.clone(),
            player_black: game.black.clone(),
            notation: "".to_string(),
            result: "0-0".to_string(),
            timertime: game.settings.time_control.contingent.as_secs() as i32,
            timerinc: game.settings.time_control.increment.as_secs() as i32,
            rating_white: game.white_rating as i32,
            rating_black: game.black_rating as i32,
            unrated: game.game_type == GameType::Unrated,
            tournament: game.game_type == GameType::Tournament,
            komi: game.settings.half_komi as i32,
            pieces: game.settings.reserve_pieces as i32,
            capstones: game.settings.reserve_capstones as i32,
            rating_change_white: -1000,
            rating_change_black: -1000,
            extra_time_amount: game
                .settings
                .time_control
                .extra
                .as_ref()
                .map_or(0, |(trigger_move, _)| *trigger_move as i32),
            extra_time_trigger: game
                .settings
                .time_control
                .extra
                .as_ref()
                .map_or(0, |(_, extra_time)| extra_time.as_secs() as i32),
        };
        self.create_game(&game_entity).await
    }

    async fn update_game(&self, id: GameId, update: &GameRecordUpdate) -> ServiceResult<()> {
        let notation_val = update
            .moves
            .iter()
            .map(|action| Self::move_to_database_string(action))
            .collect::<Vec<_>>()
            .join(" ");
        let result_val = game_state_to_string(&update.result);
        sqlx::query("UPDATE games SET notation = ?, result = ? WHERE id = ?")
            .bind(&notation_val)
            .bind(&result_val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        Ok(())
    }
}
