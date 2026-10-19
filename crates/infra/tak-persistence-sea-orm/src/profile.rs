use std::str::FromStr;

use crate::transaction::TransactionContext;
use country_code_enum::CountryCode;
use sea_orm::{EntityTrait, Iterable, sea_query::OnConflict};
use tak_domain::{
    UserId,
    profile::{AccountProfile, ProfilePictureVersion},
};
use tak_persistence_sea_orm_entities::profile;
use tak_server_app::ports::{
    profile::AccountProfileRepository,
    transaction::{TransactionError, TransactionItem},
};

pub struct ProfileRepositoryImpl;

impl AccountProfileRepository<TransactionContext> for ProfileRepositoryImpl {
    fn insert_profile_information(
        &self,
        user_id: UserId,
        profile_information: AccountProfile,
    ) -> TransactionItem<TransactionContext, Result<(), TransactionError>> {
        TransactionItem::simple(Box::new(move |ctx| {
            Box::pin(async move {
                let active_model = profile::ActiveModel {
                    user_id: sea_orm::ActiveValue::Set(user_id.0),
                    display_name: sea_orm::ActiveValue::Set(
                        profile_information.display_name.clone(),
                    ),
                    country: sea_orm::ActiveValue::Set(
                        profile_information.country.map(|x| x.to_string()),
                    ),
                    profile_picture_version: sea_orm::ActiveValue::Set(
                        profile_information.profile_picture_version.map(|x| x.0),
                    ),
                };
                profile::Entity::insert(active_model)
                    .on_conflict(
                        OnConflict::column(profile::Column::UserId)
                            .update_columns(
                                profile::Column::iter()
                                    .filter(|c| !matches!(c, profile::Column::UserId))
                                    .collect::<Vec<_>>(),
                            )
                            .to_owned(),
                    )
                    .exec(ctx)
                    .await
                    .map_err(|e| TransactionError(e.to_string()))?;
                Ok(())
            })
        }))
    }

    fn get_profile_information(
        &self,
        user_id: UserId,
    ) -> TransactionItem<TransactionContext, Result<Option<AccountProfile>, TransactionError>> {
        TransactionItem::simple(Box::new(move |ctx| {
            Box::pin(async move {
                let profile_model = profile::Entity::find_by_id(user_id.0)
                    .one(ctx)
                    .await
                    .map_err(|e| TransactionError(e.to_string()))?;

                if let Some(model) = profile_model {
                    let profile_information = AccountProfile {
                        display_name: model.display_name,
                        country: model
                            .country
                            .as_deref()
                            .and_then(|c| CountryCode::from_str(c).ok()),
                        profile_picture_version: model
                            .profile_picture_version
                            .map(ProfilePictureVersion),
                    };
                    Ok(Some(profile_information))
                } else {
                    Ok(None)
                }
            })
        }))
    }
}
