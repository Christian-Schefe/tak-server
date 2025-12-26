use std::sync::Arc;

use crate::{create_db_pool, entity::player};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set,
    TransactionError, TransactionTrait,
};
use tak_server_app::domain::{
    AccountId, PlayerId, RepoCreateError, RepoError, RepoRetrieveError, RepoUpdateError,
    player::{Player, PlayerRepository},
};

pub struct PlayerRepositoryImpl {
    db: DatabaseConnection,
    player_cache: Arc<moka::future::Cache<PlayerId, Player>>,
}

impl PlayerRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
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
            account_id: model.account_id.map(AccountId),
            is_bot: model.is_bot,
            is_silenced: model.is_silenced,
            is_banned: model.is_banned,
        }
    }

    fn db_error_to_repo_error(e: DbErr) -> RepoUpdateError {
        match e {
            sea_orm::DbErr::RecordNotFound(_) | sea_orm::DbErr::RecordNotUpdated => {
                RepoUpdateError::NotFound
            }
            e => match e.sql_err() {
                Some(
                    sea_orm::SqlErr::UniqueConstraintViolation(_)
                    | sea_orm::SqlErr::ForeignKeyConstraintViolation(_),
                ) => RepoUpdateError::Conflict,
                _ => RepoUpdateError::StorageError(e.to_string()),
            },
        }
    }
}

#[async_trait::async_trait]
impl PlayerRepository for PlayerRepositoryImpl {
    async fn create_player(&self, player: Player) -> Result<(), RepoCreateError> {
        let new_player = player::ActiveModel {
            id: Set(player.player_id.0),
            account_id: Set(player.account_id.map(|aid| aid.0)),
            is_bot: Set(player.is_bot),
            is_silenced: Set(player.is_silenced),
            is_banned: Set(player.is_banned),
        };

        new_player
            .insert(&self.db)
            .await
            .map_err(|e| match e.sql_err() {
                Some(sea_orm::SqlErr::UniqueConstraintViolation(_)) => RepoCreateError::Conflict,
                _ => RepoCreateError::StorageError(e.to_string()),
            })?;

        self.player_cache.invalidate(&player.player_id).await;
        Ok(())
    }

    async fn delete_player(&self, player_id: PlayerId) -> Result<(), RepoRetrieveError> {
        let player: player::ActiveModel = player::ActiveModel {
            id: Set(player_id.0),
            ..Default::default()
        };
        let res = player
            .delete(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?;
        self.player_cache.invalidate(&player_id).await;
        if res.rows_affected == 0 {
            Err(RepoRetrieveError::NotFound)
        } else {
            Ok(())
        }
    }

    async fn get_player(&self, player_id: PlayerId) -> Result<Player, RepoRetrieveError> {
        if let Some(player) = self.player_cache.get(&player_id).await {
            return Ok(player);
        }

        let player_model = player::Entity::find_by_id(player_id.0)
            .one(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?;

        if let Some(model) = player_model {
            let player = Self::model_to_player(model);
            self.player_cache.insert(player_id, player.clone()).await;
            Ok(player)
        } else {
            Err(RepoRetrieveError::NotFound)
        }
    }

    async fn get_or_create_player_by_account_id(
        &self,
        account_id: AccountId,
        create_fn: impl Fn() -> Player + Send + 'static,
    ) -> Result<Player, RepoError> {
        let res = self
            .db
            .transaction::<_, Player, RepoError>(|c| {
                Box::pin(async move {
                    let player_model = player::Entity::find()
                        .filter(player::Column::AccountId.eq(account_id.0))
                        .one(c)
                        .await
                        .map_err(|e| RepoError::StorageError(e.to_string()))?;

                    if let Some(model) = player_model {
                        let player = Self::model_to_player(model);
                        Ok(player)
                    } else {
                        let new_player = create_fn();
                        let active_model = player::ActiveModel {
                            id: Set(new_player.player_id.0),
                            account_id: Set(Some(account_id.0)),
                            is_bot: Set(new_player.is_bot),
                            is_silenced: Set(new_player.is_silenced),
                            is_banned: Set(new_player.is_banned),
                        };
                        let inserted_model = active_model
                            .insert(c)
                            .await
                            .map_err(|e| RepoError::StorageError(e.to_string()))?;
                        let player = Self::model_to_player(inserted_model);
                        Ok(player)
                    }
                })
            })
            .await;
        match res {
            Ok(player) => {
                self.player_cache
                    .insert(player.player_id, player.clone())
                    .await;
                Ok(player)
            }
            Err(TransactionError::Transaction(e)) => Err(e),
            Err(TransactionError::Connection(e)) => Err(RepoError::StorageError(e.to_string())),
        }
    }

    async fn link_account(
        &self,
        player_id: PlayerId,
        account_id: AccountId,
    ) -> Result<(), RepoUpdateError> {
        let player: player::ActiveModel = player::ActiveModel {
            id: Set(player_id.0),
            account_id: Set(Some(account_id.0)),
            ..Default::default()
        };
        player
            .update(&self.db)
            .await
            .map_err(Self::db_error_to_repo_error)?;
        self.player_cache.invalidate(&player_id).await;
        Ok(())
    }

    async fn unlink_account(&self, player_id: PlayerId) -> Result<(), RepoUpdateError> {
        let player: player::ActiveModel = player::ActiveModel {
            id: Set(player_id.0),
            account_id: Set(None),
            ..Default::default()
        };
        player
            .update(&self.db)
            .await
            .map_err(Self::db_error_to_repo_error)?;
        self.player_cache.invalidate(&player_id).await;
        Ok(())
    }

    async fn set_player_silenced(
        &self,
        player_id: PlayerId,
        silenced: bool,
    ) -> Result<(), RepoUpdateError> {
        let player: player::ActiveModel = player::ActiveModel {
            id: Set(player_id.0),
            is_silenced: Set(silenced),
            ..Default::default()
        };
        player
            .update(&self.db)
            .await
            .map_err(Self::db_error_to_repo_error)?;
        self.player_cache.invalidate(&player_id).await;
        Ok(())
    }

    async fn set_player_banned(
        &self,
        player_id: PlayerId,
        banned: bool,
    ) -> Result<(), RepoUpdateError> {
        let player: player::ActiveModel = player::ActiveModel {
            id: Set(player_id.0),
            is_banned: Set(banned),
            ..Default::default()
        };
        player
            .update(&self.db)
            .await
            .map_err(Self::db_error_to_repo_error)?;
        self.player_cache.invalidate(&player_id).await;
        Ok(())
    }

    async fn set_player_is_bot(
        &self,
        player_id: PlayerId,
        is_bot: bool,
    ) -> Result<(), RepoUpdateError> {
        let player: player::ActiveModel = player::ActiveModel {
            id: Set(player_id.0),
            is_bot: Set(is_bot),
            ..Default::default()
        };
        player
            .update(&self.db)
            .await
            .map_err(Self::db_error_to_repo_error)?;
        self.player_cache.invalidate(&player_id).await;
        Ok(())
    }
}
