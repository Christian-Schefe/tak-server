use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::ToSql;

use crate::{
    DatabaseError,
    game::GameId,
    persistence::{DatabaseResult, get_connection, to_sql_option, update_entry},
};

#[derive(Debug)]
pub struct GameEntity {
    pub id: i64,
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

pub struct GameUpdate {
    pub notation: Option<String>,
    pub result: Option<String>,
}

pub trait GameRepository {
    fn create_game(&self, game: &GameEntity) -> DatabaseResult<GameId>;
    fn update_game(&self, id: GameId, update: &GameUpdate) -> DatabaseResult<()>;
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
}

impl GameRepository for GameRepositoryImpl {
    fn create_game(&self, game: &GameEntity) -> DatabaseResult<GameId> {
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
        .map_err(|e| DatabaseError::QueryError(e))?;
        Ok(conn.last_insert_rowid())
    }

    fn update_game(&self, id: GameId, update: &GameUpdate) -> DatabaseResult<()> {
        let value_pairs: Vec<(&'static str, Option<&dyn ToSql>)> = vec![
            ("notation", to_sql_option(&update.notation)),
            ("result", to_sql_option(&update.result)),
        ];
        update_entry(&self.pool, "games", ("id", &id), value_pairs)
    }
}
