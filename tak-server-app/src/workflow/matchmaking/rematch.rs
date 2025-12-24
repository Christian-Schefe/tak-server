use std::sync::Arc;

use crate::{
    domain::{
        MatchId, PlayerId,
        game::GameService,
        game_history::{GameHistoryService, GameRepository},
        r#match::MatchService,
    },
    processes::game_timeout_runner::GameTimeoutRunner,
    workflow::matchmaking::create_game_from_match,
};

pub trait RematchUseCase {
    fn request_or_accept_rematch(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<(), RematchError>;
}
pub struct RematchUseCaseImpl<
    M: MatchService,
    G: GameService,
    GH: GameHistoryService,
    GR: GameRepository,
    GT: GameTimeoutRunner,
> {
    match_service: Arc<M>,
    game_service: Arc<G>,
    game_history_service: Arc<GH>,
    game_repository: Arc<GR>,
    game_timeout_runner: Arc<GT>,
}

impl<
    M: MatchService,
    G: GameService,
    GH: GameHistoryService,
    GR: GameRepository,
    GT: GameTimeoutRunner,
> RematchUseCaseImpl<M, G, GH, GR, GT>
{
    pub fn new(
        match_service: Arc<M>,
        game_service: Arc<G>,
        game_history_service: Arc<GH>,
        game_repository: Arc<GR>,
        game_timeout_runner: Arc<GT>,
    ) -> Self {
        Self {
            match_service,
            game_service,
            game_history_service,
            game_repository,
            game_timeout_runner,
        }
    }
}

pub enum RematchError {
    MatchNotFound,
    NotParticipant,
    RematchAlreadyAccepted,
}

impl<
    M: MatchService,
    G: GameService,
    GH: GameHistoryService,
    GR: GameRepository,
    GT: GameTimeoutRunner,
> RematchUseCase for RematchUseCaseImpl<M, G, GH, GR, GT>
{
    fn request_or_accept_rematch(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<(), RematchError> {
        let should_create_game = match self
            .match_service
            .request_or_accept_rematch(match_id, player)
        {
            Ok(s) => s,
            Err(crate::domain::r#match::RematchError::MatchNotFound) => {
                return Err(RematchError::MatchNotFound);
            }
            Err(crate::domain::r#match::RematchError::InvalidPlayer) => {
                return Err(RematchError::NotParticipant);
            }
        };
        if should_create_game {
            let Some(match_entry) = self.match_service.get_match(match_id) else {
                return Err(RematchError::MatchNotFound);
            };
            create_game_from_match(
                &self.match_service,
                &self.game_history_service,
                &self.game_repository,
                &self.game_service,
                &self.game_timeout_runner,
                &match_entry,
            );
        }
        Ok(())
    }
}
