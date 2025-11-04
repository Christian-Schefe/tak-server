use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::ToSql;
use tak_server_domain::{
    ServiceError, ServiceResult,
    player::{Player, PlayerFilter, PlayerId, PlayerRepository, PlayerUpdate},
};

use crate::persistence::{get_connection, to_sql_option, update_entry};

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

    fn player_from_row(row: &rusqlite::Row) -> rusqlite::Result<(PlayerId, Player)> {
        let id = row.get("id")?;
        Ok((
            id,
            Player {
                password_hash: map_string_to_option(row.get("password")?),
                username: row.get("name")?,
                rating: row.get("rating")?,
                email: map_string_to_option(row.get("email")?),
                is_bot: row.get("isbot")?,
                is_gagged: row.get("is_gagged")?,
                is_mod: row.get("is_mod")?,
                is_admin: row.get("is_admin")?,
                is_banned: row.get("is_banned")?,
            },
        ))
    }
}

fn map_string_to_option(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}

impl PlayerRepository for PlayerRepositoryImpl {
    fn get_player_by_id(&self, id: i64) -> ServiceResult<Option<Player>> {
        let conn = get_connection(&self.pool)?;
        let player = conn.query_one(
            "SELECT * FROM players WHERE id = ?1",
            [id],
            Self::player_from_row,
        );
        match player {
            Ok((_, player)) => Ok(Some(player)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(ServiceError::Internal(e.to_string())),
        }
    }

    fn get_player_by_name(&self, name: &str) -> ServiceResult<Option<(PlayerId, Player)>> {
        let conn = get_connection(&self.pool)?;
        let player = conn.query_one(
            "SELECT * FROM players WHERE name = ?1",
            [name],
            Self::player_from_row,
        );
        match player {
            Ok(player) => Ok(Some(player)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(ServiceError::Internal(e.to_string())),
        }
    }

    // TODO: remove manual id handling, use AUTOINCREMENT
    fn create_player(&self, player: &Player) -> ServiceResult<()> {
        let conn = get_connection(&self.pool)?;
        let largest_player_id: i32 = conn
            .query_row("SELECT MAX(id) FROM players", [], |row| {
                row.get::<_, i32>(0)
            })
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        conn.execute(
            "INSERT INTO players (id, name, email, password, rating, isbot, is_gagged, is_mod, is_admin, is_banned) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                largest_player_id + 1,
                player.username,
                player.email.as_ref().unwrap_or(&String::new()),
                player.password_hash.as_ref().unwrap_or(&String::new()),
                player.rating,
                player.is_bot,
                player.is_gagged,
                player.is_mod,
                player.is_admin,
                player.is_banned,
            ],
        )
        .map_err(|e| ServiceError::Internal(e.to_string()))?;
        Ok(())
    }

    fn update_player(&self, id: i64, update: &PlayerUpdate) -> ServiceResult<()> {
        let value_pairs: Vec<(&'static str, Option<&dyn ToSql>)> = vec![
            ("password", to_sql_option(&update.password_hash)),
            ("isbot", to_sql_option(&update.is_bot)),
            ("is_gagged", to_sql_option(&update.is_gagged)),
            ("is_mod", to_sql_option(&update.is_mod)),
            ("is_admin", to_sql_option(&update.is_admin)),
            ("is_banned", to_sql_option(&update.is_banned)),
        ];
        update_entry(&self.pool, "players", ("id", &id), value_pairs)
    }

    fn get_players(&self, filter: PlayerFilter) -> ServiceResult<Vec<Player>> {
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
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        let player_iter = stmt
            .query_map(
                rusqlite::params_from_iter(params.iter()),
                Self::player_from_row,
            )
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        let mut players = Vec::new();
        for player in player_iter {
            let (_, player) = player.map_err(|e| ServiceError::Internal(e.to_string()))?;
            players.push(player);
        }
        Ok(players)
    }

    fn get_player_names(&self) -> ServiceResult<Vec<String>> {
        let conn = get_connection(&self.pool)?;
        let mut stmt = conn
            .prepare("SELECT name FROM players")
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        let name_iter = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        let mut names = Vec::new();
        for name in name_iter {
            names.push(name.map_err(|e| ServiceError::Internal(e.to_string()))?);
        }
        Ok(names)
    }
}
