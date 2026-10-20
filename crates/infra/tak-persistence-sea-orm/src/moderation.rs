use sea_orm::{EntityTrait, Iterable, sea_query::OnConflict};

use tak_domain::{
    UserId,
    moderation::{Moderation, ModerationFlag, ModerationFlags, UserRole, UserType},
};
use tak_persistence_sea_orm_entities::moderation;
use tak_server_app::ports::{moderation::ModerationRepository, transaction::RepoError};

use crate::transaction::RepositoryContext;

pub struct ModerationRepositoryImpl;

impl ModerationRepositoryImpl {
    fn moderation_to_active_model(
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
    fn model_to_moderation(model: moderation::Model) -> Result<(UserId, Moderation), String> {
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
impl ModerationRepository<RepositoryContext> for ModerationRepositoryImpl {
    async fn get_moderation(
        &self,
        ctx: &RepositoryContext,
        user_id: UserId,
    ) -> Result<Option<Moderation>, RepoError> {
        let Some(model) = moderation::Entity::find_by_id(user_id.0)
            .one(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?
        else {
            return Ok(None);
        };
        let (_, moderation) =
            Self::model_to_moderation(model).map_err(|e| RepoError(e.to_string()))?;
        Ok(Some(moderation))
    }
    async fn set_moderation(
        &self,
        ctx: &RepositoryContext,
        user_id: UserId,
        moderation: Moderation,
    ) -> Result<(), RepoError> {
        let active_model = Self::moderation_to_active_model(user_id, &moderation);
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
            .exec(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;
        Ok(())
    }
}
