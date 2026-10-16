use sea_orm::{DatabaseConnection, EntityTrait, Iterable, sea_query::OnConflict};

use tak_persistence_sea_orm_entities::moderation;
use tak_domain::{
    RepoError, RepoRetrieveError, UserId,
    moderation::{
        Moderation, ModerationFlag, ModerationFlags, ModerationRepository, UserRole, UserType,
    },
};

use crate::create_db_pool;

pub struct ModerationRepositoryImpl {
    db: DatabaseConnection,
}

impl ModerationRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
        Self { db }
    }

    fn moderation_to_active_model(
        &self,
        user_id: UserId,
        moderation: &Moderation,
    ) -> moderation::ActiveModel {
        moderation::ActiveModel {
            user_id: sea_orm::ActiveValue::Set(user_id.0),
            user_type: sea_orm::ActiveValue::Set(
                match moderation.user_type {
                    UserType::Player => "player",
                    UserType::Bot => "bot",
                    UserType::Guest => "guest",
                }
                .to_string(),
            ),
            role: sea_orm::ActiveValue::Set(
                match moderation.role {
                    UserRole::Admin => "admin",
                    UserRole::Moderator => "moderator",
                    UserRole::User => "user",
                }
                .to_string(),
            ),
            banned: sea_orm::ActiveValue::Set(moderation.flags.is_flagged(ModerationFlag::Banned)),
            silenced: sea_orm::ActiveValue::Set(
                moderation.flags.is_flagged(ModerationFlag::Silenced),
            ),
        }
    }
    fn model_to_moderation(
        &self,
        model: moderation::Model,
    ) -> Result<(UserId, Moderation), String> {
        Ok((
            UserId(model.user_id),
            Moderation {
                user_type: match model.user_type.as_str() {
                    "player" => UserType::Player,
                    "bot" => UserType::Bot,
                    "guest" => UserType::Guest,
                    _ => {
                        return Err(format!("Invalid user type: {}", model.user_type));
                    }
                },
                role: match model.role.as_str() {
                    "admin" => UserRole::Admin,
                    "moderator" => UserRole::Moderator,
                    "user" => UserRole::User,
                    _ => return Err(format!("Invalid role: {}", model.role)),
                },
                flags: ModerationFlags::new(model.banned, model.silenced),
            },
        ))
    }
}

#[async_trait::async_trait]
impl ModerationRepository for ModerationRepositoryImpl {
    async fn get_moderation(&self, user_id: UserId) -> Result<Moderation, RepoRetrieveError> {
        let model = moderation::Entity::find_by_id(user_id.0)
            .one(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?
            .ok_or(RepoRetrieveError::NotFound)?;
        let (_, moderation) = self
            .model_to_moderation(model)
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?;
        Ok(moderation)
    }
    async fn set_moderation(
        &self,
        user_id: UserId,
        moderation: Moderation,
    ) -> Result<(), RepoError> {
        let active_model = self.moderation_to_active_model(user_id, &moderation);
        moderation::Entity::insert(active_model)
            .on_conflict(
                OnConflict::columns([moderation::Column::UserId])
                    .update_columns(
                        moderation::Column::iter()
                            .filter(|c| !matches!(c, moderation::Column::UserId))
                            .collect::<Vec<_>>(),
                    )
                    .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;
        Ok(())
    }
}
