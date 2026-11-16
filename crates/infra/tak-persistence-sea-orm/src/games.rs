use sea_orm::{
    ColumnTrait, EntityTrait, ExprTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use tak_core::{
    TakPlayer,
    ptn::{game_result_from_string, game_result_to_string},
};
use tak_domain::{
    GameId, MatchId, UserId,
    game_record::{GameMetadata, GameRatingInfo, GameRecord, PlayerSnapshot},
};
use tak_persistence_sea_orm_entities::game;
use tak_server_app::ports::{
    PaginatedResponse, SortOrder,
    game::{DateSelector, GameIdSelector, GamePlayerFilter, GameQuery, GameRepository, GameSortBy},
    transaction::RepoError,
};

use crate::{game_settings_from_json, game_settings_to_json, transaction::RepositoryContext};

pub struct GameRepositoryImpl;

impl GameRepositoryImpl {
    fn model_to_game(model: game::Model) -> Result<(GameId, GameRecord), String> {
        let rating_info = if let Some(rating_change_white) = model.rating_change_white
            && let Some(rating_change_black) = model.rating_change_black
        {
            Some(GameRatingInfo {
                rating_change_white: rating_change_white,
                rating_change_black: rating_change_black,
            })
        } else {
            None
        };

        let white_id = UserId(model.player_white_id);
        let black_id = UserId(model.player_black_id);

        let white_snapshot = PlayerSnapshot::new(
            white_id,
            model.player_white_username.clone(),
            model.player_white_rating,
        );

        let black_snapshot = PlayerSnapshot::new(
            black_id,
            model.player_black_username.clone(),
            model.player_black_rating,
        );

        let metadata = GameMetadata {
            date: model.date,
            white: white_snapshot,
            black: black_snapshot,
            is_rated: model.is_rated,
            settings: game_settings_from_json(&model.game_settings),
            match_id: model.match_id.map(|id| MatchId(id)),
        };

        let result = model
            .result
            .as_ref()
            .map(|res_str| {
                game_result_from_string(res_str)
                    .ok_or_else(|| format!("Invalid game result string in database: {}", res_str))
            })
            .transpose()?;

        Ok((
            GameId(model.id),
            GameRecord {
                metadata,
                rating_info,
                result,
            },
        ))
    }
    fn game_to_model(
        game_id: Option<GameId>,
        record: &GameRecord,
    ) -> Result<game::ActiveModel, String> {
        let result = record.result.map(|f| game_result_to_string(f));

        Ok(game::ActiveModel {
            id: game_id.map_or(Default::default(), |id| Set(id.0)),

            date: Set(record.metadata.date),
            player_white_id: Set(record.metadata.white.user_id.0),
            player_black_id: Set(record.metadata.black.user_id.0),
            player_white_username: Set(record.metadata.white.username.clone()),
            player_black_username: Set(record.metadata.black.username.clone()),
            player_white_rating: Set(record.metadata.white.rating),
            player_black_rating: Set(record.metadata.black.rating),
            result: Set(result),
            is_rated: Set(record.metadata.is_rated),
            rating_change_white: Set(record
                .rating_info
                .as_ref()
                .map(|info| info.rating_change_white)),
            rating_change_black: Set(record
                .rating_info
                .as_ref()
                .map(|info| info.rating_change_black)),
            game_settings: Set(game_settings_to_json(&record.metadata.settings)),
            match_id: Set(record.metadata.match_id.map(|id| id.0)),
        })
    }
}

#[async_trait::async_trait]
impl GameRepository<RepositoryContext> for GameRepositoryImpl {
    async fn get_game(
        &self,
        ctx: &RepositoryContext,
        game_id: GameId,
    ) -> Result<Option<GameRecord>, RepoError> {
        let model = game::Entity::find_by_id(game_id.0)
            .one(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;

        if let Some(model) = model {
            let (_, game) = Self::model_to_game(model)
                .map_err(|e| RepoError(format!("Failed to convert game model to record: {}", e)))?;
            Ok(Some(game))
        } else {
            Ok(None)
        }
    }
    async fn create_game(
        &self,
        ctx: &RepositoryContext,
        game: GameRecord,
    ) -> Result<GameId, RepoError> {
        let model = Self::game_to_model(None, &game)
            .map_err(|e| RepoError(format!("Failed to convert game record to model: {}", e)))?;
        let res = game::Entity::insert(model)
            .exec(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;
        let game_id = GameId(res.last_insert_id);
        Ok(game_id)
    }
    async fn update_game(
        &self,
        ctx: &RepositoryContext,
        game_id: GameId,
        game: GameRecord,
    ) -> Result<(), RepoError> {
        let model = Self::game_to_model(Some(game_id), &game)
            .map_err(|e| RepoError(format!("Failed to convert game record to model: {}", e)))?;
        game::Entity::update(model)
            .exec(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;
        Ok(())
    }
    async fn get_ongoing_games(
        &self,
        ctx: &RepositoryContext,
    ) -> Result<Vec<(GameId, GameRecord)>, RepoError> {
        let models = game::Entity::find()
            .filter(game::Column::Result.is_null())
            .all(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;

        let mut results = Vec::new();
        for model in models {
            let (game_id, game) = Self::model_to_game(model)
                .map_err(|e| RepoError(format!("Failed to convert game model to record: {}", e)))?;
            results.push((game_id, game));
        }
        Ok(results)
    }
    async fn get_ongoing_games_by_player(
        &self,
        ctx: &RepositoryContext,
        player_id: UserId,
    ) -> Result<Vec<(GameId, GameRecord)>, RepoError> {
        let models = game::Entity::find()
            .filter(
                game::Column::Result.is_null().and(
                    game::Column::PlayerWhiteId
                        .eq(player_id.0)
                        .or(game::Column::PlayerBlackId.eq(player_id.0)),
                ),
            )
            .all(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;

        let mut results = Vec::new();
        for model in models {
            let (game_id, game) = Self::model_to_game(model)
                .map_err(|e| RepoError(format!("Failed to convert game model to record: {}", e)))?;
            results.push((game_id, game));
        }
        Ok(results)
    }
    async fn query_games(
        &self,
        ctx: &RepositoryContext,
        filter: GameQuery,
    ) -> Result<PaginatedResponse<(GameId, GameRecord)>, RepoError> {
        let mut query = game::Entity::find();
        if let Some(game_id_selector) = filter.id_selector {
            query = match game_id_selector {
                GameIdSelector::Range(start_id, end_id) => {
                    query.filter(game::Column::Id.between(start_id.0, end_id.0))
                }
                GameIdSelector::AndBefore(end_id) => query.filter(game::Column::Id.lte(end_id.0)),
                GameIdSelector::AndAfter(start_id) => {
                    query.filter(game::Column::Id.gte(start_id.0))
                }
                GameIdSelector::List(id_list) => {
                    query.filter(game::Column::Id.is_in(id_list.iter().map(|id| id.0)))
                }
            }
        }
        if let Some(date_selector) = filter.date_selector {
            query = match date_selector {
                DateSelector::Range(start_date, end_date) => query.filter(
                    game::Column::Date.between(start_date.timestamp(), end_date.timestamp()),
                ),
                DateSelector::Before(end_date) => {
                    query.filter(game::Column::Date.lte(end_date.timestamp()))
                }
                DateSelector::After(start_date) => {
                    query.filter(game::Column::Date.gte(start_date.timestamp()))
                }
            }
        }
        for (filter, color) in filter.player_filters {
            let condition =
                match filter {
                    GamePlayerFilter::Contains(name_part) => {
                        let condition = sea_orm::Condition::any();
                        match color {
                            Some(TakPlayer::White) => condition
                                .add(game::Column::PlayerWhiteUsername.contains(&name_part)),
                            Some(TakPlayer::Black) => condition
                                .add(game::Column::PlayerBlackUsername.contains(&name_part)),
                            None => condition
                                .add(game::Column::PlayerWhiteUsername.contains(&name_part))
                                .add(game::Column::PlayerBlackUsername.contains(&name_part)),
                        }
                    }
                    GamePlayerFilter::Equals(name) => {
                        let condition = sea_orm::Condition::any();
                        match color {
                            Some(TakPlayer::White) => {
                                condition.add(game::Column::PlayerWhiteUsername.eq(&name))
                            }
                            Some(TakPlayer::Black) => {
                                condition.add(game::Column::PlayerBlackUsername.eq(&name))
                            }
                            None => condition
                                .add(game::Column::PlayerWhiteUsername.eq(&name))
                                .add(game::Column::PlayerBlackUsername.eq(&name)),
                        }
                    }
                    GamePlayerFilter::UserId(user_id) => {
                        let condition = sea_orm::Condition::any();
                        match color {
                            Some(TakPlayer::White) => {
                                condition.add(game::Column::PlayerWhiteId.eq(user_id.0))
                            }
                            Some(TakPlayer::Black) => {
                                condition.add(game::Column::PlayerBlackId.eq(user_id.0))
                            }
                            None => condition
                                .add(game::Column::PlayerWhiteId.eq(user_id.0))
                                .add(game::Column::PlayerBlackId.eq(user_id.0)),
                        }
                    }
                };
            query = query.filter(condition);
        }

        if let Some(is_rated) = filter.is_rated {
            query = query.filter(game::Column::IsRated.eq(is_rated));
        }

        query = query.filter(game::Column::Result.is_not_null());
        if !filter.game_results.is_empty() {
            let result_strings: Vec<String> = filter
                .game_results
                .iter()
                .map(|result| game_result_to_string(*result))
                .collect();
            query = query.filter(game::Column::Result.is_in(result_strings));
        }

        let total_count: u64 = query
            .clone()
            .count(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;

        if let Some((sort_order, sort_by)) = filter.sort {
            query = match (sort_by, sort_order) {
                (GameSortBy::Date, SortOrder::Ascending) => query.order_by_asc(game::Column::Date),
                (GameSortBy::Date, SortOrder::Descending) => {
                    query.order_by_desc(game::Column::Date)
                }
                (GameSortBy::GameId, SortOrder::Ascending) => query.order_by_asc(game::Column::Id),
                (GameSortBy::GameId, SortOrder::Descending) => {
                    query.order_by_desc(game::Column::Id)
                }
            }
        }

        if let Some(offset) = filter.pagination.offset {
            query = query.offset(offset as u64);
        }
        if let Some(limit) = filter.pagination.limit {
            query = query.limit(limit as u64);
        }

        let models = query.all(ctx).await.map_err(|e| RepoError(e.to_string()))?;

        let mut results = Vec::new();
        for model in models {
            let game_record = Self::model_to_game(model)
                .map_err(|e| RepoError(format!("Failed to convert game model to record: {}", e)))?;
            results.push(game_record);
        }

        Ok(PaginatedResponse {
            total_count: total_count as usize,
            items: results,
        })
    }

    async fn get_games_of_match(
        &self,
        ctx: &RepositoryContext,
        match_id: MatchId,
    ) -> Result<Vec<(GameId, GameRecord)>, RepoError> {
        let models = game::Entity::find()
            .filter(game::Column::MatchId.eq(match_id.0))
            .all(ctx)
            .await
            .map_err(|e| RepoError(e.to_string()))?;

        let mut results = Vec::new();
        for model in models {
            let game_record = Self::model_to_game(model)
                .map_err(|e| RepoError(format!("Failed to convert game model to record: {}", e)))?;
            results.push(game_record);
        }

        Ok(results)
    }
}
