use std::sync::Arc;

use crate::domain::{
    PlayerId, RepoError,
    rating::{PlayerRating, RatingRepository},
};

#[async_trait::async_trait]
pub trait PlayerGetRatingUseCase {
    async fn get_rating(&self, player_id: PlayerId) -> Option<PlayerRating>;
}

pub struct PlayerGetRatingUseCaseImpl<R: RatingRepository> {
    rating_repository: Arc<R>,
}

impl<R: RatingRepository> PlayerGetRatingUseCaseImpl<R> {
    pub fn new(rating_repository: Arc<R>) -> Self {
        Self { rating_repository }
    }
}

#[async_trait::async_trait]
impl<R: RatingRepository + Send + Sync + 'static> PlayerGetRatingUseCase
    for PlayerGetRatingUseCaseImpl<R>
{
    async fn get_rating(&self, player_id: PlayerId) -> Option<PlayerRating> {
        match self
            .rating_repository
            .get_or_create_player_rating(player_id, || PlayerRating::new())
            .await
        {
            Ok(rating) => Some(rating),
            Err(RepoError::StorageError(e)) => {
                log::error!(
                    "Error retrieving player rating for player {}: {}",
                    player_id,
                    e
                );
                None
            }
        }
    }
}
