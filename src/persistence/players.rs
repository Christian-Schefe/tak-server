use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::ToSql;

use crate::{
    DatabaseError,
    persistence::{DatabaseResult, get_connection, to_sql_option, update_entry},
    player::{Player, PlayerUsername},
};

#[derive(Clone, Default)]
pub struct PlayerUpdate {
    pub username: Option<PlayerUsername>,
    pub email: Option<String>,
    pub rating: Option<f64>,
    pub password_hash: Option<String>,
    pub is_bot: Option<bool>,
    pub is_gagged: Option<bool>,
    pub is_mod: Option<bool>,
    pub is_admin: Option<bool>,
    pub is_banned: Option<bool>,
}

pub struct PlayerFilter {
    pub is_bot: Option<bool>,
    pub is_gagged: Option<bool>,
    pub is_mod: Option<bool>,
    pub is_admin: Option<bool>,
    pub is_banned: Option<bool>,
}

pub trait PlayerRepository {
    fn get_player_by_id(&self, id: i64) -> DatabaseResult<Option<Player>>;
    fn get_player_by_name(&self, name: &str) -> DatabaseResult<Option<Player>>;
    fn create_player(&self, player: &Player) -> DatabaseResult<()>;
    fn update_player(&self, id: i64, update: &PlayerUpdate) -> DatabaseResult<()>;
    fn get_players(&self, filter: PlayerFilter) -> DatabaseResult<Vec<Player>>;
    fn get_player_names(&self) -> DatabaseResult<Vec<String>>;
}

pub struct PlayerRepositoryImpl {
    pool: Pool<SqliteConnectionManager>,
}

impl PlayerRepositoryImpl {
    pub fn new() -> Self {
        let db_path = std::env::var("TAK_PLAYER_DB").expect("TAK_PLAYER_DB env var not set");
        let manager = SqliteConnectionManager::file(db_path);
        let pool = Pool::builder()
            .max_size(5)
            .build(manager)
            .expect("Failed to create DB pool");
        Self { pool }
    }

    fn player_from_row(row: &rusqlite::Row) -> rusqlite::Result<Player> {
        Ok(Player {
            password_hash: row.get("password")?,
            username: row.get("name")?,
            rating: row.get("rating")?,
            id: row.get("id")?,
            email: row.get("email")?,
            is_bot: row.get("isbot")?,
            is_gagged: row.get("is_gagged")?,
            is_mod: row.get("is_mod")?,
            is_admin: row.get("is_admin")?,
            is_banned: row.get("is_banned")?,
        })
    }
}

impl PlayerRepository for PlayerRepositoryImpl {
    fn get_player_by_id(&self, id: i64) -> DatabaseResult<Option<Player>> {
        let conn = get_connection(&self.pool)?;
        let player = conn.query_one(
            "SELECT * FROM players WHERE id = ?1",
            [id],
            Self::player_from_row,
        );
        match player {
            Ok(player) => Ok(Some(player)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryError(e)),
        }
    }

    fn get_player_by_name(&self, name: &str) -> DatabaseResult<Option<Player>> {
        let conn = get_connection(&self.pool)?;
        let player = conn.query_one(
            "SELECT * FROM players WHERE name = ?1",
            [name],
            Self::player_from_row,
        );
        match player {
            Ok(player) => Ok(Some(player)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryError(e)),
        }
    }

    // TODO: remove manual id handling, use AUTOINCREMENT
    fn create_player(&self, player: &Player) -> DatabaseResult<()> {
        let conn = get_connection(&self.pool)?;
        let largest_player_id: i32 = conn
            .query_row("SELECT MAX(id) FROM players", [], |row| {
                row.get::<_, i32>(0)
            })
            .map_err(|e| DatabaseError::QueryError(e))?;
        conn.execute(
            "INSERT INTO players (id, name, email, password, rating, isbot, is_gagged, is_mod, is_admin, is_banned) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                largest_player_id + 1,
                player.username,
                player.email,
                player.password_hash,
                player.rating,
                player.is_bot,
                player.is_gagged,
                player.is_mod,
                player.is_admin,
                player.is_banned,
            ],
        )
        .map_err(|e| DatabaseError::QueryError(e))?;
        Ok(())
    }

    fn update_player(&self, id: i64, update: &PlayerUpdate) -> DatabaseResult<()> {
        let value_pairs: Vec<(&'static str, Option<&dyn ToSql>)> = vec![
            ("name", to_sql_option(&update.username)),
            ("email", to_sql_option(&update.email)),
            ("rating", to_sql_option(&update.rating)),
            ("password", to_sql_option(&update.password_hash)),
            ("isbot", to_sql_option(&update.is_bot)),
            ("is_gagged", to_sql_option(&update.is_gagged)),
            ("is_mod", to_sql_option(&update.is_mod)),
            ("is_admin", to_sql_option(&update.is_admin)),
            ("is_banned", to_sql_option(&update.is_banned)),
        ];
        update_entry(&self.pool, "players", ("id", &id), value_pairs)
    }

    fn get_players(&self, filter: PlayerFilter) -> DatabaseResult<Vec<Player>> {
        let mut query = "SELECT * FROM players".to_string();
        let mut conditions = Vec::new();
        let mut params: Vec<&dyn ToSql> = Vec::new();

        let pairs: Vec<(&'static str, Option<&dyn ToSql>)> = vec![
            ("isbot", to_sql_option(&filter.is_bot)),
            ("is_gagged", to_sql_option(&filter.is_gagged)),
            ("is_mod", to_sql_option(&filter.is_mod)),
            ("is_admin", to_sql_option(&filter.is_admin)),
            ("is_banned", to_sql_option(&filter.is_banned)),
        ];

        for (field, value) in pairs {
            if let Some(v) = value {
                conditions.push(format!("{} = ?", field));
                params.push(v);
            }
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        let conn = get_connection(&self.pool)?;
        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| DatabaseError::QueryError(e))?;
        let player_iter = stmt
            .query_map(
                rusqlite::params_from_iter(params.iter()),
                Self::player_from_row,
            )
            .map_err(|e| DatabaseError::QueryError(e))?;

        let mut players = Vec::new();
        for player in player_iter {
            players.push(player.map_err(|e| DatabaseError::QueryError(e))?);
        }
        Ok(players)
    }

    fn get_player_names(&self) -> DatabaseResult<Vec<String>> {
        let conn = get_connection(&self.pool)?;
        let mut stmt = conn
            .prepare("SELECT name FROM players")
            .map_err(|e| DatabaseError::QueryError(e))?;
        let name_iter = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| DatabaseError::QueryError(e))?;

        let mut names = Vec::new();
        for name in name_iter {
            names.push(name.map_err(|e| DatabaseError::QueryError(e))?);
        }
        Ok(names)
    }
}
