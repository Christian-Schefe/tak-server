use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use tak_auth::{AccountRepository, FullAccount};
use tak_persistence_sea_orm_entities::account;
use tak_server_app::{
    domain::{
        AccountId, RepoError, RepoRetrieveError,
        moderation::{AccountRole, ModerationFlag, ModerationFlags},
    },
    ports::authentication::AccountType,
};

use crate::create_db_pool;

pub struct AccountRepositoryImpl {
    db: DatabaseConnection,
}

impl AccountRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
        Self { db }
    }

    fn account_to_active_model(&self, account: &FullAccount) -> account::ActiveModel {
        account::ActiveModel {
            account_id: sea_orm::ActiveValue::Set(account.account_id.0),
            username: sea_orm::ActiveValue::Set(account.account.username.clone()),
            email: sea_orm::ActiveValue::Set(account.account.email.clone()),
            account_type: sea_orm::ActiveValue::Set(
                match account.account.account_type {
                    AccountType::Player => "player",
                    AccountType::Bot => "bot",
                    AccountType::Guest => "guest",
                }
                .to_string(),
            ),
            role: sea_orm::ActiveValue::Set(
                match account.account.role {
                    AccountRole::Admin => "admin",
                    AccountRole::Moderator => "moderator",
                    AccountRole::User => "user",
                }
                .to_string(),
            ),
            banned: sea_orm::ActiveValue::Set(
                account.account.flags.is_flagged(ModerationFlag::Banned),
            ),
            silenced: sea_orm::ActiveValue::Set(
                account.account.flags.is_flagged(ModerationFlag::Silenced),
            ),
            password_hash: sea_orm::ActiveValue::Set(account.password_hash.clone()),
        }
    }
    fn model_to_account(&self, model: account::Model) -> Result<FullAccount, String> {
        Ok(FullAccount {
            account_id: AccountId(model.account_id),
            account: tak_server_app::ports::authentication::Account {
                account_type: match model.account_type.as_str() {
                    "player" => AccountType::Player,
                    "bot" => AccountType::Bot,
                    "guest" => AccountType::Guest,
                    _ => {
                        return Err(format!("Invalid account type: {}", model.account_type));
                    }
                },
                role: match model.role.as_str() {
                    "admin" => AccountRole::Admin,
                    "moderator" => AccountRole::Moderator,
                    "user" => AccountRole::User,
                    _ => return Err(format!("Invalid role: {}", model.role)),
                },
                flags: ModerationFlags::new(model.banned, model.silenced),
                username: model.username,
                email: model.email,
            },
            password_hash: model.password_hash,
        })
    }
}

#[async_trait::async_trait]
impl AccountRepository for AccountRepositoryImpl {
    async fn create_account(&self, account: FullAccount) -> Result<(), RepoError> {
        let active_model = self.account_to_active_model(&account);
        account::Entity::insert(active_model)
            .exec(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;
        Ok(())
    }

    async fn get_account(&self, account_id: &AccountId) -> Result<FullAccount, RepoRetrieveError> {
        let model = account::Entity::find_by_id(account_id.0)
            .one(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?
            .ok_or(RepoRetrieveError::NotFound)?;
        self.model_to_account(model)
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))
    }

    async fn update_account(&self, account: FullAccount) -> Result<(), RepoError> {
        let active_model = self.account_to_active_model(&account);
        account::Entity::update(active_model)
            .exec(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;
        Ok(())
    }

    async fn get_account_by_username(
        &self,
        username: &str,
    ) -> Result<FullAccount, RepoRetrieveError> {
        let model = account::Entity::find()
            .filter(account::Column::Username.eq(username))
            .one(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?
            .ok_or(RepoRetrieveError::NotFound)?;
        self.model_to_account(model)
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))
    }

    async fn get_account_by_email(&self, email: &str) -> Result<FullAccount, RepoRetrieveError> {
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
