use crate::transaction::RepositoryContext;
use crate::{
    match_settings_from_json, match_settings_to_json, match_status_from_json, match_status_to_json,
};
use sea_orm::ActiveValue::{NotSet, Set};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tak_core::TakPlayer;
use tak_domain::matches::{Match, MatchPlayer, MatchTournamentInfo};
use tak_domain::{MatchId, TournamentId, UserId};
use tak_persistence_sea_orm_entities::matches;
use tak_server_app::ports::matches::MatchRepository;
use tak_server_app::ports::transaction::RepoError;

pub struct MatchRepositoryImpl;

impl MatchRepositoryImpl {
    fn model_to_match(model: matches::Model) -> Result<(MatchId, Match), String> {
        Ok((
            MatchId(model.match_id),
            Match {
                player1: MatchPlayer {
                    user_id: UserId(model.player1_id),
                    score: model.score_player1 as u32,
                },
                player2: MatchPlayer {
                    user_id: UserId(model.player2_id),
                    score: model.score_player2 as u32,
                },
                initial_color: match model.initial_color.as_str() {
                    "white" => Ok(TakPlayer::White),
                    "black" => Ok(TakPlayer::Black),
                    _ => Err(format!(
                        "Invalid initial color in database: {}",
                        model.initial_color
                    )),
                }?,
                settings: match_settings_from_json(&model.settings),
                status: match_status_from_json(&model.status),
                games_played: model.games_played as u32,
                tournament_info: if let Some(tournament_id) = model.tournament_id
                    && let Some(round) = model.tournament_round
                    && let Some(match_number) = model.tournament_round_match_number
                {
                    Some(MatchTournamentInfo {
                        tournament_id: TournamentId(tournament_id),
                        round: round as u32,
                        round_match_number: match_number as u32,
                    })
                } else {
                    None
                },
            },
        ))
    }

    fn match_to_model(
        match_id: Option<MatchId>,
        match_entry: &Match,
    ) -> Result<matches::ActiveModel, String> {
        Ok(matches::ActiveModel {
            match_id: match_id.map(|id| Set(id.0)).unwrap_or(NotSet),
            player1_id: Set(match_entry.player1.user_id.0),
            player2_id: Set(match_entry.player2.user_id.0),
            initial_color: Set(match match_entry.initial_color {
                TakPlayer::White => "white".to_string(),
                TakPlayer::Black => "black".to_string(),
            }),
            settings: Set(match_settings_to_json(&match_entry.settings)),
            status: Set(match_status_to_json(&match_entry.status)),
            games_played: Set(match_entry.games_played as i32),
            score_player1: Set(match_entry.player1.score as i32),
            score_player2: Set(match_entry.player2.score as i32),
            tournament_id: Set(match_entry
                .tournament_info
                .as_ref()
                .map(|info| info.tournament_id.0)),
            tournament_round: Set(match_entry
                .tournament_info
                .as_ref()
                .map(|info| info.round as i64)),
            tournament_round_match_number: Set(match_entry
                .tournament_info
                .as_ref()
                .map(|info| info.round_match_number as i64)),
        })
    }
}

#[async_trait::async_trait]
impl MatchRepository<RepositoryContext> for MatchRepositoryImpl {
    async fn create_match(
        &self,
        ctx: &RepositoryContext,
        new_match: Match,
    ) -> Result<MatchId, RepoError> {
        let active_model = Self::match_to_model(None, &new_match)
            .map_err(|e| RepoError(format!("Failed to convert match to model: {}", e)))?;
        match matches::Entity::insert(active_model).exec(ctx).await {
            Ok(inserted) => Ok(MatchId(inserted.last_insert_id)),
            Err(e) => Err(RepoError(format!(
                "Failed to insert match into database: {}",
                e
            ))),
        }
    }

    async fn get_match(
        &self,
        ctx: &RepositoryContext,
        match_id: MatchId,
    ) -> Result<Option<Match>, RepoError> {
        match matches::Entity::find_by_id(match_id.0).one(ctx).await {
            Ok(Some(model)) => Self::model_to_match(model)
                .map_err(|e| RepoError(format!("Failed to convert model to match: {}", e)))
                .map(|(_, match_entry)| Some(match_entry)),
            Ok(None) => Ok(None),
            Err(e) => Err(RepoError(format!(
                "Failed to query match from database: {}",
                e
            ))),
        }
    }

    async fn update_match(
        &self,
        ctx: &RepositoryContext,
        match_id: MatchId,
        updated_match: Match,
    ) -> Result<(), RepoError> {
        let active_model = Self::match_to_model(Some(match_id), &updated_match)
            .map_err(|e| RepoError(format!("Failed to convert match to model: {}", e)))?;
        match matches::Entity::update(active_model).exec(ctx).await {
            Ok(_) => Ok(()),
            Err(e) => Err(RepoError(format!(
                "Failed to update match in database: {}",
                e
            ))),
        }
    }

    async fn get_matches_of_tournament(
        &self,
        ctx: &RepositoryContext,
        tournament_id: TournamentId,
    ) -> Result<Vec<(MatchId, Match)>, RepoError> {
        match matches::Entity::find()
            .filter(matches::Column::TournamentId.eq(tournament_id.0))
            .all(ctx)
            .await
        {
            Ok(models) => models
                .into_iter()
                .map(Self::model_to_match)
                .collect::<Result<Vec<_>, String>>()
                .map_err(|e| RepoError(format!("Failed to convert model to match: {}", e))),
            Err(e) => Err(RepoError(format!(
                "Failed to query matches from database: {}",
                e
            ))),
        }
    }
}
