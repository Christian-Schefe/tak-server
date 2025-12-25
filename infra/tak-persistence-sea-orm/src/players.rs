use std::sync::Arc;

use crate::{create_player_db_pool, entity::player};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use tak_server_app::domain::{
    AccountId, PlayerId,
    player::{CreatePlayerError, Player, PlayerRepoError, PlayerRepository},
};

pub struct PlayerRepositoryImpl {
    db: DatabaseConnection,
    player_cache: Arc<moka::future::Cache<PlayerId, Player>>,
}

impl PlayerRepositoryImpl {
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

    fn model_to_player(model: player::Model) -> Player {
        Player {
            player_id: PlayerId(model.id),
            is_bot: model.is_bot,
            is_silenced: model.is_silenced,
            is_banned: model.is_banned,
        }
    }
}

#[async_trait::async_trait]
impl PlayerRepository for PlayerRepositoryImpl {
    async fn create_player(
        &self,
        player: Player,
        account_id: Option<AccountId>,
    ) -> Result<(), CreatePlayerError> {
        let new_player = player::ActiveModel {
            id: Set(player.player_id.0),
            account_id: Set(account_id.map(|aid| aid.0)),
            is_bot: Set(player.is_bot),
            is_silenced: Set(player.is_silenced),
            is_banned: Set(player.is_banned),
        };

        new_player.insert(&self.db).await.map_err(|e| match e {
            sea_orm::DbErr::RecordNotInserted => CreatePlayerError::PlayerAlreadyExists,
            _ => CreatePlayerError::StorageError(e.to_string()),
        })?;

        self.player_cache.invalidate(&player.player_id).await;
        Ok(())
    }

    async fn delete_player(&self, player_id: PlayerId) -> Result<bool, PlayerRepoError> {
        let player: player::ActiveModel = player::ActiveModel {
            id: Set(player_id.0),
            ..Default::default()
        };
        let res = player
            .delete(&self.db)
            .await
            .map_err(|e| PlayerRepoError::StorageError(e.to_string()))?;
        self.player_cache.invalidate(&player_id).await;
        Ok(res.rows_affected > 0)
    }

    async fn get_player(&self, player_id: PlayerId) -> Result<Option<Player>, PlayerRepoError> {
        if let Some(player) = self.player_cache.get(&player_id).await {
            return Ok(Some(player));
        }

        let player_model = player::Entity::find_by_id(player_id.0)
            .one(&self.db)
            .await
            .map_err(|e| PlayerRepoError::StorageError(e.to_string()))?;

        if let Some(model) = player_model {
            let player = Self::model_to_player(model);
            self.player_cache.insert(player_id, player.clone()).await;
            Ok(Some(player))
        } else {
            Ok(None)
        }
    }

    async fn link_account(
        &self,
        player_id: PlayerId,
        account_id: AccountId,
    ) -> Result<(), PlayerRepoError> {
        let player: player::ActiveModel = player::ActiveModel {
            id: Set(player_id.0),
            account_id: Set(Some(account_id.0)),
            ..Default::default()
        };
        player
            .update(&self.db)
            .await
            .map_err(|e| PlayerRepoError::StorageError(e.to_string()))?;
        self.player_cache.invalidate(&player_id).await;
        Ok(())
    }

    async fn unlink_account(&self, player_id: PlayerId) -> Result<(), PlayerRepoError> {
        let player: player::ActiveModel = player::ActiveModel {
            id: Set(player_id.0),
            account_id: Set(None),
            ..Default::default()
        };
        player
            .update(&self.db)
            .await
            .map_err(|e| PlayerRepoError::StorageError(e.to_string()))?;
        self.player_cache.invalidate(&player_id).await;
        Ok(())
    }

    async fn get_player_by_account_id(
        &self,
        account_id: AccountId,
    ) -> Result<Option<Player>, PlayerRepoError> {
        let player_model = player::Entity::find_by_account_id(account_id.0)
            .one(&self.db)
            .await
            .map_err(|e| PlayerRepoError::StorageError(e.to_string()))?;

        if let Some(model) = player_model {
            let player = Self::model_to_player(model);
            self.player_cache
                .insert(player.player_id, player.clone())
                .await;
            Ok(Some(player))
        } else {
            Ok(None)
        }
    }

    async fn get_account_id_for_player(&self, player_id: PlayerId) -> Option<AccountId> {}

    async fn set_player_silenced(&self, player_id: PlayerId, silenced: bool) {}

    async fn set_player_banned(&self, player_id: PlayerId, banned: bool) {}

    async fn set_player_is_bot(&self, player_id: PlayerId, is_bot: bool) {}
}
