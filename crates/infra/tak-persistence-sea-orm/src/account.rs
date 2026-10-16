use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use tak_auth::{Account, AccountRepository};
use tak_domain::{RepoError, RepoRetrieveError, UserId};
use tak_persistence_sea_orm_entities::account;

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
    fn model_to_account(&self, model: account::Model) -> Result<Account, String> {
        Ok(Account {
            user_id: UserId(model.user_id),
            username: model.username,
            email: model.email,
            password_hash: model.password_hash,
            unique_name: model.unique_name,
        })
    }
}

#[async_trait::async_trait]
impl AccountRepository for AccountRepositoryImpl {
    async fn create_account(&self, account: Account) -> Result<(), RepoError> {
        let active_model = self.account_to_active_model(&account);
        account::Entity::insert(active_model)
            .exec(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;
        Ok(())
    }

    async fn get_account(&self, user_id: UserId) -> Result<Account, RepoRetrieveError> {
        let model = account::Entity::find_by_id(user_id.0)
            .one(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?
            .ok_or(RepoRetrieveError::NotFound)?;
        self.model_to_account(model)
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))
    }

    async fn update_account(&self, account: Account) -> Result<(), RepoError> {
        let active_model = self.account_to_active_model(&account);
        account::Entity::update(active_model)
            .exec(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;
        Ok(())
    }

    async fn get_account_by_username(&self, username: &str) -> Result<Account, RepoRetrieveError> {
        let model = account::Entity::find()
            .filter(account::Column::Username.eq(username))
            .one(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?
            .ok_or(RepoRetrieveError::NotFound)?;
        self.model_to_account(model)
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))
    }

    async fn get_account_by_email(&self, email: &str) -> Result<Account, RepoRetrieveError> {
        let model = account::Entity::find()
            .filter(account::Column::Email.eq(email))
            .one(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?
            .ok_or(RepoRetrieveError::NotFound)?;
        self.model_to_account(model)
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))
    }
}
