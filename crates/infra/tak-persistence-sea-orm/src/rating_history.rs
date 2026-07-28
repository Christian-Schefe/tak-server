use crate::create_db_pool;
use chrono::{DateTime, Utc};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QueryTrait,
    sea_query::OnConflict,
};
use tak_persistence_sea_orm_entities::rating_history;
use tak_server_app::domain::{
    PlayerId, RepoError,
    stats::{RatingHistoryEntry, RatingHistoryRange, RatingHistoryRepository},
};

pub struct RatingHistoryRepositoryImpl {
    db: DatabaseConnection,
}

impl RatingHistoryRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
        Self { db }
    }

    fn model_to_rating(model: rating_history::Model) -> RatingHistoryEntry {
        RatingHistoryEntry {
            timestamp: model.timestamp,
            rating: model.rating,
        }
    }

    fn rating_to_model(
        player_id: PlayerId,
        rating: &RatingHistoryEntry,
    ) -> rating_history::ActiveModel {
        rating_history::ActiveModel {
            player_id: sea_orm::Set(player_id.0),
            timestamp: sea_orm::Set(rating.timestamp),
            rating: sea_orm::Set(rating.rating),
        }
    }
}

#[async_trait::async_trait]
impl RatingHistoryRepository for RatingHistoryRepositoryImpl {
    async fn get_rating_history(
        &self,
        player_id: PlayerId,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<RatingHistoryRange, RepoError> {
        let entries: Vec<RatingHistoryEntry> = rating_history::Entity::find()
            .filter(rating_history::Column::PlayerId.eq(player_id.0))
            .apply_if(from, |query, from| {
                query.filter(rating_history::Column::Timestamp.gte(from))
            })
            .apply_if(to, |query, to| {
                query.filter(rating_history::Column::Timestamp.lte(to))
            })
            .order_by_desc(rating_history::Column::Timestamp)
            .all(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(format!("Failed to retrieve rating history: {e}")))
            .map(|models| models.into_iter().map(Self::model_to_rating).collect())?;

        let first_entry_before_range = if let Some(from) = from {
            let first_value_outside_range = rating_history::Entity::find()
                .filter(rating_history::Column::PlayerId.eq(player_id.0))
                .filter(rating_history::Column::Timestamp.lt(from))
                .order_by_desc(rating_history::Column::Timestamp)
                .one(&self.db)
                .await
                .map_err(|e| {
                    RepoError::StorageError(format!("Failed to retrieve rating history: {e}"))
                })?;
            first_value_outside_range.map(Self::model_to_rating)
        } else {
            None
        };
        Ok(RatingHistoryRange {
            entries,
            first_entry_before_range,
        })
    }
    async fn add_rating_history_entry(
        &self,
        player_id: PlayerId,
        rating: RatingHistoryEntry,
    ) -> Result<(), RepoError> {
        let model = Self::rating_to_model(player_id, &rating);
        rating_history::Entity::insert(model)
            .on_conflict(
                OnConflict::columns([
                    rating_history::Column::PlayerId,
                    rating_history::Column::Timestamp,
                ])
                .update_column(rating_history::Column::Rating)
                .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(|e| {
                RepoError::StorageError(format!("Failed to add rating history entry: {e}"))
            })?;
        Ok(())
    }
}
