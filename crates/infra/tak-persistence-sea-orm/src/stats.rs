use crate::transaction::RepositoryContext;
use sea_orm::sea_query::OnConflict;
use sea_orm::{EntityTrait, Iterable};
use tak_domain::{UserId, stats::PlayerStats};
use tak_persistence_sea_orm_entities::stats;
use tak_server_app::ports::stats::StatsRepository;
use tak_server_app::ports::transaction::RepoError;

pub struct StatsRepositoryImpl;

impl StatsRepositoryImpl {
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

    fn stats_to_model(user_id: UserId, stats: &PlayerStats) -> stats::ActiveModel {
        stats::ActiveModel {
            user_id: sea_orm::Set(user_id.0),
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
impl StatsRepository<RepositoryContext> for StatsRepositoryImpl {
    async fn get_player_stats(
        &self,
        ctx: &RepositoryContext,
        user_id: UserId,
    ) -> Result<Option<PlayerStats>, RepoError> {
        let Some(model) = stats::Entity::find_by_id(user_id.0)
            .one(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?
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
    ) -> Result<(), RepoError> {
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
            .map_err(|e| RepoError(e.to_string()))?;
        Ok(())
    }

    async fn remove_player_stats(
        &self,
        ctx: &RepositoryContext,
        user_id: UserId,
    ) -> Result<(), RepoError> {
        match stats::Entity::delete_by_id(user_id.0).exec(ctx).await {
            Ok(_) => Ok(()),
            Err(e) => Err(RepoError(e.to_string())),
        }
    }
}
