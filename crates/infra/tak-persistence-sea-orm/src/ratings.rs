use std::collections::HashMap;

use crate::transaction::RepositoryContext;
use sea_orm::{
    ColumnTrait, EntityTrait, Iterable, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    sea_query::OnConflict,
};
use tak_domain::{UserId, rating::PlayerRating};
use tak_persistence_sea_orm_entities::rating;
use tak_server_app::ports::{
    PaginatedResponse, SortOrder,
    rating::{RatingQuery, RatingRepository, RatingSortBy},
    transaction::RepoError,
};

pub struct RatingRepositoryImpl;

impl RatingRepositoryImpl {
    fn model_to_rating(model: rating::Model) -> PlayerRating {
        PlayerRating {
            user_id: UserId(model.user_id),
            rating: model.rating,
            boost: model.boost,
            max_rating: model.max_rating,
            rated_games_played: model.rated_games as u32,
            rating_age: model.rating_age,
            fatigue: serde_json::from_value::<HashMap<uuid::Uuid, f64>>(model.fatigue)
                .unwrap_or_default()
                .into_iter()
                .map(|(k, v)| (UserId(k), v))
                .collect(),
        }
    }

    fn rating_to_model(user_id: UserId, rating: &PlayerRating) -> rating::ActiveModel {
        rating::ActiveModel {
            user_id: sea_orm::Set(user_id.0),
            rating: sea_orm::Set(rating.rating),
            boost: sea_orm::Set(rating.boost),
            max_rating: sea_orm::Set(rating.max_rating),
            rated_games: sea_orm::Set(rating.rated_games_played as i32),
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
impl RatingRepository<RepositoryContext> for RatingRepositoryImpl {
    async fn get_player_rating(
        &self,
        ctx: &RepositoryContext,
        user_id: UserId,
    ) -> Result<Option<PlayerRating>, RepoError> {
        let rating_model = rating::Entity::find_by_id(user_id.0)
            .one(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;
        match rating_model {
            Some(model) => Ok(Some(Self::model_to_rating(model))),
            None => Ok(None),
        }
    }

    async fn get_player_ranking(
        &self,
        ctx: &RepositoryContext,
        user_id: UserId,
    ) -> Result<Option<(u32, PlayerRating)>, RepoError> {
        let rating_model = rating::Entity::find_by_id(user_id.0)
            .one(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;
        match rating_model {
            Some(model) => {
                let player_rating = Self::model_to_rating(model);
                let better_count = rating::Entity::find()
                    .filter(rating::Column::Rating.gt(player_rating.rating))
                    .count(ctx)
                    .await
                    .map_err(|e| RepoError(e.to_string()))?;
                Ok(Some(((better_count + 1) as u32, player_rating)))
            }
            None => Ok(None),
        }
    }

    async fn query_player_ratings(
        &self,
        ctx: &RepositoryContext,
        filter: RatingQuery,
    ) -> Result<PaginatedResponse<PlayerRating>, RepoError> {
        let mut query = rating::Entity::find();

        let total_count = query
            .clone()
            .count(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;

        if let Some((sort_order, sort_by)) = filter.sort {
            let order = match sort_order {
                SortOrder::Ascending => sea_orm::Order::Asc,
                SortOrder::Descending => sea_orm::Order::Desc,
            };
            match sort_by {
                RatingSortBy::Rating => {
                    query = query.order_by(rating::Column::Rating, order);
                }
                RatingSortBy::RatedGames => {
                    query = query.order_by(rating::Column::RatedGames, order);
                }
                RatingSortBy::MaxRating => {
                    query = query.order_by(rating::Column::MaxRating, order);
                }
            }
        } else {
            query = query.order_by_desc(rating::Column::Rating);
        }
        if let Some(offset) = filter.pagination.offset {
            query = query.offset(offset as u64);
        }
        if let Some(limit) = filter.pagination.limit {
            query = query.limit(limit as u64);
        }

        let items = query
            .all(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?
            .into_iter()
            .map(Self::model_to_rating)
            .collect::<Vec<_>>();

        Ok(PaginatedResponse {
            total_count: total_count as usize,
            items,
        })
    }

    async fn set_player_rating(
        &self,
        ctx: &RepositoryContext,
        user_id: UserId,
        rating: PlayerRating,
    ) -> Result<(), RepoError> {
        let active_model = Self::rating_to_model(user_id, &rating);
        rating::Entity::insert(active_model)
            .on_conflict(
                OnConflict::column(rating::Column::UserId)
                    .update_columns(
                        rating::Column::iter()
                            .filter(|c| !matches!(c, rating::Column::UserId))
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
