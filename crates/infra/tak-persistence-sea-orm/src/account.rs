use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, SqlErr};
use tak_auth::{Account, AccountRepository};
use tak_domain::UserId;
use tak_persistence_sea_orm_entities::account;
use tak_server_app::errors::{AppResult, EmailTaken, InternalError, OneOf};

use crate::create_db_pool;

pub struct AccountRepositoryImpl {
    db: DatabaseConnection,
}

impl AccountRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
        Self { db }
    }

    fn account_to_active_model(&self, account: &Account) -> account::ActiveModel {
        account::ActiveModel {
            user_id: sea_orm::ActiveValue::Set(account.user_id.0),
            username: sea_orm::ActiveValue::Set(account.username.clone()),
            email: sea_orm::ActiveValue::Set(account.email.clone()),
            password_hash: sea_orm::ActiveValue::Set(account.password_hash.clone()),
            unique_name: sea_orm::ActiveValue::Set(account.unique_name.clone()),
        }
    }
    fn model_to_account(&self, model: account::Model) -> Account {
        Account {
            user_id: UserId(model.user_id),
            username: model.username,
            email: model.email,
            password_hash: model.password_hash,
            unique_name: model.unique_name,
        }
    }
}

#[async_trait::async_trait]
impl AccountRepository for AccountRepositoryImpl {
    async fn create_account(&self, account: Account) -> AppResult<(), (EmailTaken, InternalError)> {
        let active_model = self.account_to_active_model(&account);
        account::Entity::insert(active_model)
            .exec(&self.db)
            .await
            .map_err(|e| match e.sql_err() {
                Some(SqlErr::UniqueConstraintViolation(_)) => OneOf::new(EmailTaken),
                _ => {
                    tracing::error!("Failed to create account: {}", e);
                    OneOf::new(InternalError)
                }
            })?;
        Ok(())
    }

    async fn get_account(&self, user_id: UserId) -> AppResult<Option<Account>, (InternalError,)> {
        let Some(model) = account::Entity::find_by_id(user_id.0)
            .one(&self.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get account: {}", e);
                OneOf::new(InternalError)
            })?
        else {
            return Ok(None);
        };
        Ok(Some(self.model_to_account(model)))
    }

    async fn update_account(&self, account: Account) -> AppResult<(), (InternalError,)> {
        let active_model = self.account_to_active_model(&account);
        account::Entity::update(active_model)
            .exec(&self.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update account: {}", e);
                OneOf::new(InternalError)
            })?;
        Ok(())
    }

    async fn get_account_by_username(
        &self,
        username: &str,
    ) -> AppResult<Option<Account>, (InternalError,)> {
        let Some(model) = account::Entity::find()
            .filter(account::Column::Username.eq(username))
            .one(&self.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get account by username: {}", e);
                OneOf::new(InternalError)
            })?
        else {
            return Ok(None);
        };
        Ok(Some(self.model_to_account(model)))
    }

    async fn get_account_by_email(
        &self,
        email: &str,
    ) -> AppResult<Option<Account>, (InternalError,)> {
        let Some(model) = account::Entity::find()
            .filter(account::Column::Email.eq(email))
            .one(&self.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get account by email: {}", e);
                OneOf::new(InternalError)
            })?
        else {
            return Ok(None);
        };
        Ok(Some(self.model_to_account(model)))
    }
}
