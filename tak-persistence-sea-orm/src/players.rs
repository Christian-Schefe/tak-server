use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, Set,
};
use tak_server_domain::{
    ServiceError, ServiceResult,
    player::{
        Player, PlayerFilter, PlayerFlags, PlayerFlagsUpdate, PlayerId, PlayerRepositoryImpl,
    },
};

use crate::{create_player_db_pool, entity::player};

pub struct SqlitePlayerRepository {
    db: DatabaseConnection,
}

impl SqlitePlayerRepository {
    pub async fn new() -> Self {
        let db = create_player_db_pool().await;
        Self { db }
    }

    fn model_to_player(model: player::Model) -> Player {
        Player {
            password_hash: map_string_to_option(model.password),
            username: model.name,
            rating: model.rating,
            email: map_string_to_option(model.email),
            flags: PlayerFlags {
                is_bot: model.isbot,
                is_gagged: model.is_gagged,
                is_mod: model.is_mod,
                is_admin: model.is_admin,
                is_banned: model.is_banned,
            },
        }
    }
}

fn map_string_to_option(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}

#[async_trait::async_trait]
impl PlayerRepositoryImpl for SqlitePlayerRepository {
    async fn get_player_by_id(&self, id: i64) -> ServiceResult<Option<Player>> {
        let model = player::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(model.map(Self::model_to_player))
    }

    async fn get_player_by_name(&self, name: &str) -> ServiceResult<Option<(PlayerId, Player)>> {
        let model = player::Entity::find()
            .filter(player::Column::Name.eq(name))
            .one(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(model.map(|m| {
            let id = m.id;
            (id, Self::model_to_player(m))
        }))
    }

    // TODO: remove manual id handling, use AUTOINCREMENT
    async fn create_player(&self, player: &Player) -> ServiceResult<()> {
        let largest_player_id = player::Entity::find()
            .select_only()
            .column_as(player::Column::Id.max(), "max_id")
            .into_tuple::<Option<i64>>()
            .one(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?
            .flatten()
            .unwrap_or(0);

        let new_player = player::ActiveModel {
            id: Set(largest_player_id + 1),
            name: Set(player.username.clone()),
            email: Set(player.email.as_ref().unwrap_or(&String::new()).clone()),
            password: Set(player
                .password_hash
                .as_ref()
                .unwrap_or(&String::new())
                .clone()),
            rating: Set(player.rating as f64),
            boost: Set(750.0),
            ratedgames: Set(0),
            maxrating: Set(1000.0),
            ratingage: Set(0.0),
            ratingbase: Set(0),
            unrated: Set(0),
            isbot: Set(player.flags.is_bot),
            fatigue: Set("{}".to_string()),
            is_gagged: Set(player.flags.is_gagged),
            is_mod: Set(player.flags.is_mod),
            is_admin: Set(player.flags.is_admin),
            is_banned: Set(player.flags.is_banned),
            participation_rating: Set(1000),
        };

        new_player
            .insert(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn update_password(&self, id: i64, new_password_hash: String) -> ServiceResult<()> {
        let player = player::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?
            .ok_or_else(|| ServiceError::Internal("Player not found".to_string()))?;

        let mut player: player::ActiveModel = player.into();
        player.password = Set(new_password_hash);
        player
            .update(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn update_flags(&self, id: i64, update: &PlayerFlagsUpdate) -> ServiceResult<()> {
        let player = player::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?
            .ok_or_else(|| ServiceError::Internal("Player not found".to_string()))?;

        let mut player: player::ActiveModel = player.into();

        if let Some(value) = update.is_bot {
            player.isbot = Set(value);
        }
        if let Some(value) = update.is_gagged {
            player.is_gagged = Set(value);
        }
        if let Some(value) = update.is_mod {
            player.is_mod = Set(value);
        }
        if let Some(value) = update.is_admin {
            player.is_admin = Set(value);
        }
        if let Some(value) = update.is_banned {
            player.is_banned = Set(value);
        }

        player
            .update(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn get_players(&self, filter: PlayerFilter) -> ServiceResult<Vec<Player>> {
        let mut query = player::Entity::find();

        if let Some(value) = filter.is_bot {
            query = query.filter(player::Column::Isbot.eq(value));
        }
        if let Some(value) = filter.is_gagged {
            query = query.filter(player::Column::IsGagged.eq(value));
        }
        if let Some(value) = filter.is_mod {
            query = query.filter(player::Column::IsMod.eq(value));
        }
        if let Some(value) = filter.is_admin {
            query = query.filter(player::Column::IsAdmin.eq(value));
        }
        if let Some(value) = filter.is_banned {
            query = query.filter(player::Column::IsBanned.eq(value));
        }

        let models = query
            .all(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(models.into_iter().map(Self::model_to_player).collect())
    }

    async fn get_player_names(&self) -> ServiceResult<Vec<String>> {
        let names = player::Entity::find()
            .select_only()
            .column(player::Column::Name)
            .into_tuple::<String>()
            .all(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(names)
    }
}
