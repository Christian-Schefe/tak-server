use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::ToSql;
use tak_core::{TakAction, TakPos, TakVariant, ptn::game_state_to_string};
use tak_server_domain::{
    ServiceError, ServiceResult,
    game::{GameId, GameRecord, GameRecordUpdate, GameRepository, GameType},
};

use crate::persistence::{get_connection, to_sql_option, update_entry};

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

pub struct GameRepositoryImpl {
    pool: Pool<SqliteConnectionManager>,
}

impl GameRepositoryImpl {
    pub fn new() -> Self {
        let db_path = std::env::var("TAK_GAMES_DB").expect("TAK_GAMES_DB env var not set");
        let manager = SqliteConnectionManager::file(db_path);
        let pool = Pool::builder()
            .max_size(5)
            .build(manager)
            .expect("Failed to create DB pool");
        Self { pool }
    }

    fn create_game(&self, game: &GameEntity) -> ServiceResult<GameId> {
        let conn = get_connection(&self.pool)?;
        // Id is auto-incremented
        conn.execute(
            "INSERT INTO games (date, size, player_white, player_black, notation, result, timertime, timerinc, rating_white, rating_black, unrated, tournament, komi, pieces, capstones, rating_change_white, rating_change_black, extra_time_amount, extra_time_trigger) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            rusqlite::params![
                game.date,
                game.size,
                game.player_white,
                game.player_black,
                game.notation,
                game.result,
                game.timertime,
                game.timerinc,
                game.rating_white,
                game.rating_black,
                if game.unrated { 1 } else { 0 },
                if game.tournament { 1 } else { 0 },
                game.komi,
                game.pieces,
                game.capstones,
                game.rating_change_white,
                game.rating_change_black,
                game.extra_time_amount,
                game.extra_time_trigger,
            ],
        )
        .map_err(|e| ServiceError::Internal(e.to_string()))?;
        Ok(conn.last_insert_rowid())
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

impl GameRepository for GameRepositoryImpl {
    fn create_game(&self, game: &GameRecord) -> ServiceResult<GameId> {
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
        self.create_game(&game_entity)
    }

    fn update_game(&self, id: GameId, update: &GameRecordUpdate) -> ServiceResult<()> {
        let notation_val = Some(
            update
                .moves
                .iter()
                .map(|action| Self::move_to_database_string(action))
                .collect::<Vec<_>>()
                .join(" "),
        );
        let result_val = Some(game_state_to_string(&update.result));
        let value_pairs: Vec<(&'static str, Option<&dyn ToSql>)> = vec![
            ("notation", to_sql_option(&notation_val)),
            ("result", to_sql_option(&result_val)),
        ];
        update_entry(&self.pool, "games", ("id", &id), value_pairs)
    }
}
