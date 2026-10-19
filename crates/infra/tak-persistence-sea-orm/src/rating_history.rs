use crate::transaction::TransactionContext;
use chrono::{DateTime, Utc};
use sea_orm::{
    ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QueryTrait, sea_query::OnConflict,
};
use tak_domain::{
    UserId,
    stats::{RatingHistoryEntry, RatingHistoryRange},
};
use tak_persistence_sea_orm_entities::rating_history;
use tak_server_app::ports::{
    stats::RatingHistoryRepository,
    transaction::{TransactionError, TransactionItem},
};

pub struct RatingHistoryRepositoryImpl;

impl RatingHistoryRepositoryImpl {
    fn model_to_rating(model: rating_history::Model) -> RatingHistoryEntry {
        RatingHistoryEntry {
            timestamp: model.timestamp,
            rating: model.rating,
        }
    }

    fn rating_to_model(
        user_id: UserId,
        rating: &RatingHistoryEntry,
    ) -> rating_history::ActiveModel {
        rating_history::ActiveModel {
            user_id: sea_orm::Set(user_id.0),
            timestamp: sea_orm::Set(rating.timestamp),
            rating: sea_orm::Set(rating.rating),
        }
    }
}

impl RatingHistoryRepository<TransactionContext> for RatingHistoryRepositoryImpl {
    fn get_rating_history(
        &self,
        user_id: UserId,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> TransactionItem<TransactionContext, Result<RatingHistoryRange, TransactionError>> {
        TransactionItem::simple(Box::new(move |ctx| {
            Box::pin(async move {
                let entries: Vec<RatingHistoryEntry> = rating_history::Entity::find()
                    .filter(rating_history::Column::UserId.eq(user_id.0))
                    .apply_if(from, |query, from| {
                        query.filter(rating_history::Column::Timestamp.gte(from))
                    })
                    .apply_if(to, |query, to| {
                        query.filter(rating_history::Column::Timestamp.lte(to))
                    })
                    .order_by_desc(rating_history::Column::Timestamp)
                    .all(ctx)
                    .await
                    .map_err(|e| {
                        TransactionError(format!("Failed to retrieve rating history: {e}"))
                    })
                    .map(|models| models.into_iter().map(Self::model_to_rating).collect())?;

                let first_entry_before_range = if let Some(from) = from {
                    let first_value_outside_range = rating_history::Entity::find()
                        .filter(rating_history::Column::UserId.eq(user_id.0))
                        .filter(rating_history::Column::Timestamp.lt(from))
                        .order_by_desc(rating_history::Column::Timestamp)
                        .one(ctx)
                        .await
                        .map_err(|e| {
                            TransactionError(format!("Failed to retrieve rating history: {e}"))
                        })?;
                    first_value_outside_range.map(Self::model_to_rating)
                } else {
                    None
                };
                Ok(RatingHistoryRange {
                    entries,
                    first_entry_before_range,
                })
            })
        }))
    }
    fn add_rating_history_entry(
        &self,
        user_id: UserId,
        rating: RatingHistoryEntry,
    ) -> TransactionItem<TransactionContext, Result<(), TransactionError>> {
        TransactionItem::simple(Box::new(move |ctx| {
            Box::pin(async move {
                let model = Self::rating_to_model(user_id, &rating);
                rating_history::Entity::insert(model)
                    .on_conflict(
                        OnConflict::columns([
                            rating_history::Column::UserId,
                            rating_history::Column::Timestamp,
                        ])
                        .update_column(rating_history::Column::Rating)
                        .to_owned(),
                    )
                    .exec(ctx)
                    .await
                    .map_err(|e| {
                        TransactionError(format!("Failed to add rating history entry: {e}"))
                    })?;
                Ok(())
            })
        }))
    }
}
