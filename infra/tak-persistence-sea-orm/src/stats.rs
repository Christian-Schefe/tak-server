use std::sync::Arc;

use crate::create_db_pool;
use sea_orm::ActiveValue::Set;
use sea_orm::QueryFilter;
use sea_orm::prelude::Expr;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait};
use tak_persistence_sea_orm_entites::stats;
use tak_server_app::domain::{
    PlayerId, RepoError, RepoRetrieveError,
    stats::{GameOutcome, PlayerStats, StatsRepository},
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
                .time_to_live(std::time::Duration::from_secs(60 * 60 * 12))
                .build(),
        );
        Self { db, stats_cache }
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
        };

        self.stats_cache.insert(player_id, player_stats.clone());
        Ok(player_stats)
    }

    async fn update_player_game(
        &self,
        player_id: PlayerId,
        result: GameOutcome,
        was_rated: bool,
    ) -> Result<(), RepoError> {
        let mut query = stats::Entity::update_many().col_expr(
            stats::Column::GamesPlayed,
            Expr::col(stats::Column::GamesPlayed).add(1),
        );
        if was_rated {
            query = query.col_expr(
                stats::Column::RatedGamesPlayed,
                Expr::col(stats::Column::RatedGamesPlayed).add(1),
            );
        }
        match result {
            GameOutcome::Win => {
                query = query.col_expr(
                    stats::Column::GamesWon,
                    Expr::col(stats::Column::GamesWon).add(1),
                );
            }
            GameOutcome::Loss => {
                query = query.col_expr(
                    stats::Column::GamesLost,
                    Expr::col(stats::Column::GamesLost).add(1),
                );
            }
            GameOutcome::Draw => {
                query = query.col_expr(
                    stats::Column::GamesDrawn,
                    Expr::col(stats::Column::GamesDrawn).add(1),
                );
            }
        }
        match query
            .filter(stats::Column::PlayerId.eq(player_id.0))
            .exec(&self.db)
            .await
        {
            Ok(res) => {
                if res.rows_affected == 0 {
                    let default_model = stats::ActiveModel {
                        player_id: Set(player_id.0),
                        rated_games_played: Set(if was_rated { 1 } else { 0 }),
                        games_played: Set(1),
                        games_won: Set(if result == GameOutcome::Win { 1 } else { 0 }),
                        games_lost: Set(if result == GameOutcome::Loss { 1 } else { 0 }),
                        games_drawn: Set(if result == GameOutcome::Draw { 1 } else { 0 }),
                    };
                    default_model
                        .insert(&self.db)
                        .await
                        .map_err(|e| RepoError::StorageError(e.to_string()))?;
                }
                self.stats_cache.invalidate(&player_id);
                Ok(())
            }
            Err(e) => Err(RepoError::StorageError(e.to_string())),
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
