use sqlx::{Pool, Row, Sqlite, sqlite::SqliteRow};
use tak_server_domain::{
    ServiceError, ServiceResult,
    player::{Player, PlayerFilter, PlayerFlags, PlayerFlagsUpdate, PlayerId, PlayerRepository},
};

use crate::create_player_db_pool;

pub struct SqlitePlayerRepository {
    pool: Pool<Sqlite>,
}

impl SqlitePlayerRepository {
    pub fn new() -> Self {
        let pool = create_player_db_pool();
        Self { pool }
    }

    fn player_from_row(row: &SqliteRow) -> sqlx::Result<(PlayerId, Player)> {
        let id = row.try_get("id")?;
        Ok((
            id,
            Player {
                password_hash: map_string_to_option(row.try_get("password")?),
                username: row.try_get("name")?,
                rating: row.try_get("rating")?,
                email: map_string_to_option(row.try_get("email")?),
                flags: PlayerFlags {
                    is_bot: row.try_get("isbot")?,
                    is_gagged: row.try_get("is_gagged")?,
                    is_mod: row.try_get("is_mod")?,
                    is_admin: row.try_get("is_admin")?,
                    is_banned: row.try_get("is_banned")?,
                },
            },
        ))
    }
}

fn map_string_to_option(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}

#[async_trait::async_trait]
impl PlayerRepository for SqlitePlayerRepository {
    async fn get_player_by_id(&self, id: i64) -> ServiceResult<Option<Player>> {
        let player = sqlx::query("SELECT * FROM players WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        let player = Self::player_from_row(&player);
        match player {
            Ok((_, player)) => Ok(Some(player)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(ServiceError::Internal(e.to_string())),
        }
    }

    async fn get_player_by_name(&self, name: &str) -> ServiceResult<Option<(PlayerId, Player)>> {
        let player = sqlx::query("SELECT * FROM players WHERE name = ?")
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        let player = Self::player_from_row(&player);
        match player {
            Ok(player) => Ok(Some(player)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(ServiceError::Internal(e.to_string())),
        }
    }

    // TODO: remove manual id handling, use AUTOINCREMENT
    async fn create_player(&self, player: &Player) -> ServiceResult<()> {
        let largest_player_id = match sqlx::query_scalar::<_, i32>("SELECT MAX(id) FROM players")
            .fetch_one(&self.pool)
            .await
        {
            Ok(id) => id,
            Err(sqlx::Error::RowNotFound) => 0,
            Err(e) => return Err(ServiceError::Internal(e.to_string())),
        };

        let fields = vec![
            "id",
            "name",
            "email",
            "password",
            "rating",
            "isbot",
            "is_gagged",
            "is_mod",
            "is_admin",
            "is_banned",
        ];

        sqlx::query(&format!(
            "INSERT INTO players ({}) VALUES ({})",
            fields.join(", "),
            fields.iter().map(|_| "?").collect::<Vec<_>>().join(", ")
        ))
        .bind(largest_player_id + 1)
        .bind(&player.username)
        .bind(player.email.as_ref().unwrap_or(&String::new()))
        .bind(player.password_hash.as_ref().unwrap_or(&String::new()))
        .bind(player.rating)
        .bind(player.flags.is_bot)
        .bind(player.flags.is_gagged)
        .bind(player.flags.is_mod)
        .bind(player.flags.is_admin)
        .bind(player.flags.is_banned)
        .execute(&self.pool)
        .await
        .map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn update_password(&self, id: i64, new_password_hash: String) -> ServiceResult<()> {
        sqlx::query("UPDATE players SET password = ? WHERE id = ?")
            .bind(new_password_hash)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn update_flags(&self, id: i64, update: &PlayerFlagsUpdate) -> ServiceResult<()> {
        let query_str = format!("UPDATE players SET {} WHERE id = ?", {
            let mut sets = Vec::new();

            if update.is_bot.is_some() {
                sets.push("isbot = ?");
            }
            if update.is_gagged.is_some() {
                sets.push("is_gagged = ?");
            }
            if update.is_mod.is_some() {
                sets.push("is_mod = ?");
            }
            if update.is_admin.is_some() {
                sets.push("is_admin = ?");
            }
            if update.is_banned.is_some() {
                sets.push("is_banned = ?");
            }
            sets.join(", ")
        });
        let mut query = sqlx::query(&query_str);
        for field in [
            update.is_bot,
            update.is_gagged,
            update.is_mod,
            update.is_admin,
            update.is_banned,
        ] {
            if let Some(value) = field {
                query = query.bind(value);
            }
        }
        query = query.bind(id);
        query
            .execute(&self.pool)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn get_players(&self, filter: PlayerFilter) -> ServiceResult<Vec<Player>> {
        let mut query = "SELECT * FROM players".to_string();
        let mut conditions = Vec::new();
        let mut params = Vec::new();

        let pairs = [
            ("isbot", filter.is_bot),
            ("is_gagged", filter.is_gagged),
            ("is_mod", filter.is_mod),
            ("is_admin", filter.is_admin),
            ("is_banned", filter.is_banned),
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

        let mut query = sqlx::query(&query);
        for param in params {
            query = query.bind(param);
        }
        let player_iter = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        player_iter
            .into_iter()
            .map(|row| {
                Self::player_from_row(&row)
                    .map(|(_, player)| player)
                    .map_err(|e| ServiceError::Internal(e.to_string()))
            })
            .collect::<ServiceResult<Vec<Player>>>()
    }

    async fn get_player_names(&self) -> ServiceResult<Vec<String>> {
        sqlx::query_scalar::<_, String>("SELECT name FROM players")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))
    }
}
