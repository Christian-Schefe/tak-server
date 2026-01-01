use std::{collections::HashMap, sync::Arc};

use crate::create_db_pool;
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryOrder, QuerySelect,
    TransactionError, TransactionTrait,
};
use tak_persistence_sea_orm_entites::rating;
use tak_server_app::domain::{
    PaginatedResponse, PlayerId, RepoError, RepoUpdateError, SortOrder,
    rating::{PlayerRating, RatingQuery, RatingRepository, RatingSortBy},
};

pub struct RatingRepositoryImpl {
    db: DatabaseConnection,
    ratings_cache: Arc<moka::sync::Cache<PlayerId, PlayerRating>>,
}

impl RatingRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
        let ratings_cache = Arc::new(
            moka::sync::Cache::builder()
                .max_capacity(10_000)
                .time_to_live(std::time::Duration::from_secs(60 * 60))
                .build(),
        );
        Self { db, ratings_cache }
    }

    fn model_to_rating(model: rating::Model) -> PlayerRating {
        PlayerRating {
            player_id: PlayerId(model.player_id),
            rating: model.rating,
            boost: model.boost,
            max_rating: model.max_rating,
            rated_games_played: model.rated_games as u32,
            is_unrated: model.is_unrated,
            participation_rating: model.participation_rating,
            rating_age: model.rating_age,
            fatigue: serde_json::from_value::<HashMap<uuid::Uuid, f64>>(model.fatigue)
                .unwrap_or_default()
                .into_iter()
                .map(|(k, v)| (PlayerId(k), v))
                .collect(),
        }
    }

    fn rating_to_model(player_id: PlayerId, rating: &PlayerRating) -> rating::ActiveModel {
        rating::ActiveModel {
            player_id: sea_orm::Set(player_id.0),
            rating: sea_orm::Set(rating.rating),
            boost: sea_orm::Set(rating.boost),
            max_rating: sea_orm::Set(rating.max_rating),
            rated_games: sea_orm::Set(rating.rated_games_played as i32),
            is_unrated: sea_orm::Set(rating.is_unrated),
            participation_rating: sea_orm::Set(rating.participation_rating),
            rating_age: sea_orm::Set(rating.rating_age),
            fatigue: sea_orm::Set(
                serde_json::to_value(
                    &rating
                        .fatigue
                        .iter()
                        .map(|(k, v)| (k.0, *v))
                        .collect::<HashMap<uuid::Uuid, f64>>(),
                )
                .unwrap_or_else(|_| serde_json::json!({})),
            ),
        }
    }
}

#[async_trait::async_trait]
impl RatingRepository for RatingRepositoryImpl {
    async fn get_or_create_player_rating(
        &self,
        player_id: PlayerId,
        create_fn: impl Fn() -> PlayerRating + Send + 'static,
    ) -> Result<PlayerRating, RepoError> {
        if let Some(cached) = self.ratings_cache.get(&player_id) {
            return Ok(cached);
        }
        let res = self
            .db
            .transaction::<_, PlayerRating, RepoError>(|c| {
                Box::pin(async move {
                    let rating_model = rating::Entity::find_by_id(player_id.0)
                        .one(c)
                        .await
                        .map_err(|e| RepoError::StorageError(e.to_string()))?;

                    if let Some(model) = rating_model {
                        let player_rating = Self::model_to_rating(model);
                        return Ok(player_rating);
                    } else {
                        let new_rating = create_fn();
                        let new_model = Self::rating_to_model(player_id, &new_rating);
                        new_model
                            .insert(c)
                            .await
                            .map_err(|e| RepoError::StorageError(e.to_string()))?;
                        Ok(new_rating)
                    }
                })
            })
            .await;
        match res {
            Ok(player_rating) => {
                self.ratings_cache.insert(player_id, player_rating.clone());
                Ok(player_rating)
            }
            Err(TransactionError::Transaction(e)) => Err(e),
            Err(TransactionError::Connection(e)) => Err(RepoError::StorageError(e.to_string())),
        }
    }

    async fn update_player_ratings<R: Send + 'static>(
        &self,
        white: PlayerId,
        black: PlayerId,
        calc_fn: impl FnOnce(PlayerRating, PlayerRating) -> (PlayerRating, PlayerRating, R)
        + Send
        + 'static,
    ) -> Result<R, RepoUpdateError> {
        let res = self
            .db
            .transaction::<_, R, RepoUpdateError>(|c| {
                Box::pin(async move {
                    let white_rating_model = rating::Entity::find_by_id(white.0)
                        .one(c)
                        .await
                        .map_err(|e| RepoUpdateError::StorageError(e.to_string()))?;
                    let black_rating_model = rating::Entity::find_by_id(black.0)
                        .one(c)
                        .await
                        .map_err(|e| RepoUpdateError::StorageError(e.to_string()))?;

                    let white_rating = if let Some(model) = white_rating_model {
                        Self::model_to_rating(model)
                    } else {
                        return Err(RepoUpdateError::NotFound);
                    };
                    let black_rating = if let Some(model) = black_rating_model {
                        Self::model_to_rating(model)
                    } else {
                        return Err(RepoUpdateError::NotFound);
                    };

                    let (new_white_rating, new_black_rating, res) =
                        calc_fn(white_rating, black_rating);

                    let white_active_model = Self::rating_to_model(white, &new_white_rating);
                    let black_active_model = Self::rating_to_model(black, &new_black_rating);

                    white_active_model
                        .update(c)
                        .await
                        .map_err(|e| RepoUpdateError::StorageError(e.to_string()))?;
                    black_active_model
                        .update(c)
                        .await
                        .map_err(|e| RepoUpdateError::StorageError(e.to_string()))?;

                    Ok(res)
                })
            })
            .await;

        match res {
            Ok(result) => {
                self.ratings_cache.invalidate(&white);
                self.ratings_cache.invalidate(&black);
                Ok(result)
            }
            Err(TransactionError::Transaction(e)) => Err(e),
            Err(TransactionError::Connection(e)) => {
                Err(RepoUpdateError::StorageError(e.to_string()))
            }
        }
    }

    async fn query_ratings(
        &self,
        query: RatingQuery,
    ) -> Result<PaginatedResponse<PlayerRating>, RepoError> {
        let mut db_query = rating::Entity::find();

        let total_count = db_query
            .clone()
            .count(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;

        if let Some((order, sort_by)) = query.sort {
            use sea_orm::Order;
            let order_expr = match sort_by {
                RatingSortBy::Rating => rating::Column::Rating,
                RatingSortBy::MaxRating => rating::Column::MaxRating,
                RatingSortBy::ParticipationRating => rating::Column::ParticipationRating,
                RatingSortBy::RatedGames => rating::Column::RatedGames,
            };
            db_query = match order {
                SortOrder::Ascending => db_query.order_by(order_expr, Order::Asc),
                SortOrder::Descending => db_query.order_by(order_expr, Order::Desc),
            };
        }

        if let Some(limit) = query.pagination.limit {
            db_query = db_query.limit(limit as u64);
        }
        if let Some(offset) = query.pagination.offset {
            db_query = db_query.offset(offset as u64);
        }

        let models = db_query
            .all(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;

        let ratings = models
            .into_iter()
            .map(|model| Self::model_to_rating(model))
            .collect();

        Ok(PaginatedResponse {
            total_count: total_count as usize,
            items: ratings,
        })
    }
}
