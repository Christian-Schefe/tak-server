use crate::transaction::RepositoryContext;
use sea_orm::sea_query::OnConflict;
use sea_orm::{EntityTrait, Iterable};
use tak_domain::{UserId, stats::PlayerStats};
use tak_persistence_sea_orm_entities::stats;
use tak_server_app::errors::{AppResult, InternalError, OneOf};
use tak_server_app::ports::stats::StatsRepository;

pub struct StatsRepositoryImpl;

impl StatsRepositoryImpl {
    fn model_to_stats(model: stats::Model) -> PlayerStats {
        PlayerStats {
            rated_games_played: model.rated_games_played as u32,
            games_played: model.games_played as u32,
            games_won: model.games_won as u32,
            games_lost: model.games_lost as u32,
            games_drawn: model.games_drawn as u32,
            win_streak: model.win_streak as u32,
            longest_win_streak: model.longest_win_streak as u32,
        }
    }

    fn stats_to_model(user_id: UserId, stats: &PlayerStats) -> stats::ActiveModel {
        stats::ActiveModel {
            user_id: sea_orm::Set(user_id.0),
            rated_games_played: sea_orm::Set(stats.rated_games_played as i32),
            games_played: sea_orm::Set(stats.games_played as i32),
            games_won: sea_orm::Set(stats.games_won as i32),
            games_lost: sea_orm::Set(stats.games_lost as i32),
            games_drawn: sea_orm::Set(stats.games_drawn as i32),
            win_streak: sea_orm::Set(stats.win_streak as i32),
            longest_win_streak: sea_orm::Set(stats.longest_win_streak as i32),
        }
    }
}

#[async_trait::async_trait]
impl StatsRepository<RepositoryContext> for StatsRepositoryImpl {
    async fn get_player_stats(
        &self,
        ctx: &RepositoryContext,
        user_id: UserId,
    ) -> AppResult<Option<PlayerStats>, (InternalError,)> {
        let Some(model) = stats::Entity::find_by_id(user_id.0)
            .one(ctx)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to retrieve player stats for user {}: {}",
                    user_id.0,
                    e
                );
                OneOf::new(InternalError)
            })?
        else {
            return Ok(None);
        };

        let player_stats = Self::model_to_stats(model);
        Ok(Some(player_stats))
    }

    async fn set_player_stats(
        &self,
        ctx: &RepositoryContext,
        user_id: UserId,
        stats: PlayerStats,
    ) -> AppResult<(), (InternalError,)> {
        let active_model = Self::stats_to_model(user_id, &stats);
        stats::Entity::insert(active_model)
            .on_conflict(
                OnConflict::column(stats::Column::UserId)
                    .update_columns(
                        stats::Column::iter()
                            .filter(|c| !matches!(c, stats::Column::UserId))
                            .collect::<Vec<_>>(),
                    )
                    .to_owned(),
            )
            .exec(ctx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to set player stats for user {}: {}", user_id.0, e);
                OneOf::new(InternalError)
            })?;
        Ok(())
    }

    async fn remove_player_stats(
        &self,
        ctx: &RepositoryContext,
        user_id: UserId,
    ) -> AppResult<(), (InternalError,)> {
        match stats::Entity::delete_by_id(user_id.0).exec(ctx).await {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!(
                    "Failed to remove player stats for user {}: {}",
                    user_id.0,
                    e
                );
                Err(OneOf::new(InternalError))
            }
        }
    }
}
