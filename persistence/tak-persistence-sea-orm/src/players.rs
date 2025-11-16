use std::sync::Arc;

use crate::{create_player_db_pool, entity::player};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};
use tak_server_domain::{
    ServiceError, ServiceResult,
    player::{
        Player, PlayerFilter, PlayerFilterResult, PlayerFlags, PlayerFlagsUpdate, PlayerId,
        PlayerRepository, PlayerUsername,
    },
    rating::PlayerRating,
};

pub struct PlayerRepositoryImpl<C> {
    db: C,
    player_cache: Arc<moka::future::Cache<PlayerUsername, (PlayerId, Player)>>,
}

impl PlayerRepositoryImpl<DatabaseConnection> {
    pub async fn new() -> Self {
        let db = create_player_db_pool().await;
        let player_cache = Arc::new(
            moka::future::Cache::builder()
                .max_capacity(10_000)
                .time_to_live(std::time::Duration::from_secs(60 * 60))
                .build(),
        );
        Self { db, player_cache }
    }
}

impl<C> PlayerRepositoryImpl<C>
where
    C: ConnectionTrait,
{
    fn model_to_player(model: player::Model) -> (PlayerId, Player) {
        let id = model.id;
        let player = Player {
            password_hash: map_string_to_option(model.password_hash),
            username: model.name,
            rating: PlayerRating {
                rating: model.rating,
                boost: model.boost,
                max_rating: model.max_rating,
                rated_games_played: model.rated_games as u32,
                is_unrated: model.is_unrated,
                participation_rating: model.participation_rating as f64,
                rating_age: model.rating_age,
                fatigue: serde_json::from_str(&model.fatigue).unwrap_or_default(),
            },
            email: map_string_to_option(model.email),
            flags: PlayerFlags {
                is_bot: model.is_bot,
                is_silenced: model.is_silenced,
                is_mod: model.is_mod,
                is_admin: model.is_admin,
                is_banned: model.is_banned,
            },
        };
        (id, player)
    }
}

fn map_string_to_option(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}

#[async_trait::async_trait]
impl<C> PlayerRepository for PlayerRepositoryImpl<C>
where
    C: ConnectionTrait + TransactionTrait,
{
    async fn get_player_by_name(&self, name: &str) -> ServiceResult<Option<(PlayerId, Player)>> {
        if let Some(cached) = self.player_cache.get(name).await {
            return Ok(Some(cached));
        }

        let model = player::Entity::find()
            .filter(player::Column::Name.eq(name))
            .one(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?
            .map(Self::model_to_player);

        if let Some((id, ref player)) = model {
            self.player_cache
                .insert(name.to_string(), (id, player.clone()))
                .await;
        }

        Ok(model)
    }

    async fn create_player(&self, player: &Player) -> ServiceResult<()> {
        let new_player = player::ActiveModel {
            id: Default::default(), // Auto-increment
            name: Set(player.username.clone()),
            email: Set(player.email.as_ref().unwrap_or(&String::new()).clone()),
            password_hash: Set(player
                .password_hash
                .as_ref()
                .unwrap_or(&String::new())
                .clone()),
            rating: Set(player.rating.rating),
            boost: Set(player.rating.boost),
            rated_games: Set(player.rating.rated_games_played as i32),
            max_rating: Set(player.rating.max_rating),
            rating_age: Set(player.rating.rating_age),
            is_unrated: Set(player.rating.is_unrated),
            is_bot: Set(player.flags.is_bot),
            fatigue: Set(serde_json::to_string(&player.rating.fatigue).unwrap()),
            is_silenced: Set(player.flags.is_silenced),
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

        let username = player.name.clone();

        let mut player: player::ActiveModel = player.into();
        player.password_hash = Set(new_password_hash);
        player
            .update(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        self.player_cache.invalidate(&username).await;

        Ok(())
    }

    async fn update_flags(&self, id: i64, update: &PlayerFlagsUpdate) -> ServiceResult<()> {
        let player = player::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?
            .ok_or_else(|| ServiceError::Internal("Player not found".to_string()))?;

        let username = player.name.clone();
        let mut player: player::ActiveModel = player.into();

        if let Some(value) = update.is_bot {
            player.is_bot = Set(value);
        }
        if let Some(value) = update.is_silenced {
            player.is_silenced = Set(value);
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

        self.player_cache.invalidate(&username).await;

        Ok(())
    }

    async fn update_ratings(&self, items: Vec<(PlayerId, PlayerRating)>) -> ServiceResult<()> {
        let usernames = self
            .db
            .transaction::<_, _, DbErr>(|tx| {
                Box::pin(async move {
                    let mut usernames = Vec::new();
                    for (id, rating) in items {
                        let player = player::Entity::find_by_id(id)
                            .one(tx)
                            .await?
                            .ok_or_else(|| DbErr::Custom("Player not found".to_string()))?;

                        usernames.push(player.name.clone());
                        let mut player: player::ActiveModel = player.into();

                        player.rating = Set(rating.rating);
                        player.boost = Set(rating.boost);
                        player.max_rating = Set(rating.max_rating);
                        player.rated_games = Set(rating.rated_games_played as i32);
                        player.is_unrated = Set(rating.is_unrated);
                        player.participation_rating = Set(rating.participation_rating as i32);
                        player.rating_age = Set(rating.rating_age);
                        player.fatigue = Set(serde_json::to_string(&rating.fatigue).unwrap());

                        player.update(tx).await?;
                    }
                    Ok(usernames)
                })
            })
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        for username in usernames {
            self.player_cache.invalidate(&username).await;
        }
        Ok(())
    }

    async fn get_players(&self, filter: PlayerFilter) -> ServiceResult<PlayerFilterResult> {
        let mut query = player::Entity::find();

        if let Some(value) = filter.id {
            query = query.filter(player::Column::Id.eq(value));
        }
        if let Some(value) = filter.username {
            query = query.filter(player::Column::Name.eq(value));
        }
        if let Some(value) = filter.is_bot {
            query = query.filter(player::Column::IsBot.eq(value));
        }
        if let Some(value) = filter.is_silenced {
            query = query.filter(player::Column::IsSilenced.eq(value));
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

        let count_query: u64 = query
            .clone()
            .count(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        if let Some(offset) = filter.pagination.offset {
            query = query.offset(offset as u64);
        }
        if let Some(limit) = filter.pagination.limit {
            query = query.limit(limit as u64);
        }

        let models = query
            .all(&self.db)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        let players = models.into_iter().map(Self::model_to_player).collect();

        Ok(PlayerFilterResult {
            players,
            total_count: count_query as usize,
        })
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
