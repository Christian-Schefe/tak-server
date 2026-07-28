use std::sync::Arc;

use crate::create_db_pool;
use sea_orm::TransactionTrait;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, TransactionError};
use tak_persistence_sea_orm_entities::stats;
use tak_server_app::domain::{
    PlayerId, RepoError, RepoRetrieveError,
    stats::{PlayerStats, StatsRepository},
};

pub struct StatsRepositoryImpl {
    db: DatabaseConnection,
    stats_cache: Arc<moka::sync::Cache<PlayerId, PlayerStats>>,
}

impl StatsRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
        let stats_cache = Arc::new(
            moka::sync::Cache::builder()
                .max_capacity(10_000)
                .time_to_live(std::time::Duration::from_secs(60 * 5))
                .build(),
        );
        Self { db, stats_cache }
    }

    fn model_to_stats(model: stats::Model) -> PlayerStats {
        PlayerStats {
            rated_games_played: model.rated_games_played,
            games_played: model.games_played,
            games_won: model.games_won,
            games_lost: model.games_lost,
            games_drawn: model.games_drawn,
            win_streak: model.win_streak,
            longest_win_streak: model.longest_win_streak,
        }
    }

    fn stats_to_model(player_id: PlayerId, stats: &PlayerStats) -> stats::ActiveModel {
        stats::ActiveModel {
            player_id: sea_orm::Set(player_id.0),
            rated_games_played: sea_orm::Set(stats.rated_games_played),
            games_played: sea_orm::Set(stats.games_played),
            games_won: sea_orm::Set(stats.games_won),
            games_lost: sea_orm::Set(stats.games_lost),
            games_drawn: sea_orm::Set(stats.games_drawn),
            win_streak: sea_orm::Set(stats.win_streak),
            longest_win_streak: sea_orm::Set(stats.longest_win_streak),
        }
    }
}

#[async_trait::async_trait]
impl StatsRepository for StatsRepositoryImpl {
    async fn get_player_stats(
        &self,
        player_id: PlayerId,
    ) -> Result<PlayerStats, RepoRetrieveError> {
        if let Some(cached_stats) = self.stats_cache.get(&player_id) {
            return Ok(cached_stats);
        }

        let model = stats::Entity::find_by_id(player_id.0)
            .one(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?
            .ok_or(RepoRetrieveError::NotFound)?;

        let player_stats = PlayerStats {
            rated_games_played: model.rated_games_played,
            games_played: model.games_played,
            games_won: model.games_won,
            games_lost: model.games_lost,
            games_drawn: model.games_drawn,
            win_streak: model.win_streak,
            longest_win_streak: model.longest_win_streak,
        };

        self.stats_cache.insert(player_id, player_stats.clone());
        Ok(player_stats)
    }

    async fn update_player_game(
        &self,
        player_id: PlayerId,
        calc_fn: impl FnOnce(Option<PlayerStats>) -> PlayerStats + Send + 'static,
    ) -> Result<(), RepoError> {
        let res = self
            .db
            .transaction::<_, (), RepoError>(|c| {
                Box::pin(async move {
                    let stats_model = stats::Entity::find_by_id(player_id.0)
                        .one(c)
                        .await
                        .map_err(|e| RepoError::StorageError(e.to_string()))?;

                    let stats = stats_model.map(|model| Self::model_to_stats(model));

                    let has_stats = stats.is_some();

                    let new_stats = calc_fn(stats);

                    let active_model = Self::stats_to_model(player_id, &new_stats);

                    if has_stats {
                        active_model
                            .update(c)
                            .await
                            .map_err(|e| RepoError::StorageError(e.to_string()))?;
                    } else {
                        active_model
                            .insert(c)
                            .await
                            .map_err(|e| RepoError::StorageError(e.to_string()))?;
                    }
                    Ok(())
                })
            })
            .await;

        match res {
            Ok(result) => {
                self.stats_cache.invalidate(&player_id);
                Ok(result)
            }
            Err(TransactionError::Transaction(e)) => Err(e),
            Err(TransactionError::Connection(e)) => Err(RepoError::StorageError(e.to_string())),
        }
    }

    async fn remove_player_stats(&self, player_id: PlayerId) -> Result<(), RepoError> {
        match stats::Entity::delete_by_id(player_id.0)
            .exec(&self.db)
            .await
        {
            Ok(_) => {
                self.stats_cache.invalidate(&player_id);
                Ok(())
            }
            Err(e) => Err(RepoError::StorageError(e.to_string())),
        }
    }
}
