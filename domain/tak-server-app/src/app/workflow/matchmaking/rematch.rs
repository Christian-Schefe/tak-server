use std::sync::Arc;

use crate::app::{
    domain::{
        MatchId, PlayerId,
        game::GameService,
        game_history::{GameHistoryService, GameRepository},
        r#match::MatchService,
    },
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
> {
    match_service: Arc<M>,
    game_service: Arc<G>,
    game_history_service: Arc<GH>,
    game_repository: Arc<GR>,
}

impl<M: MatchService, G: GameService, GH: GameHistoryService, GR: GameRepository>
    RematchUseCaseImpl<M, G, GH, GR>
{
    pub fn new(
        match_service: Arc<M>,
        game_service: Arc<G>,
        game_history_service: Arc<GH>,
        game_repository: Arc<GR>,
    ) -> Self {
        Self {
            match_service,
            game_service,
            game_history_service,
            game_repository,
        }
    }
}

pub enum RematchError {
    MatchNotFound,
    NotParticipant,
    RematchAlreadyAccepted,
}

impl<M: MatchService, G: GameService, GH: GameHistoryService, GR: GameRepository> RematchUseCase
    for RematchUseCaseImpl<M, G, GH, GR>
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
            Err(crate::app::domain::r#match::RematchError::MatchNotFound) => {
                return Err(RematchError::MatchNotFound);
            }
            Err(crate::app::domain::r#match::RematchError::InvalidPlayer) => {
                return Err(RematchError::NotParticipant);
            }
        };
        if should_create_game {
            let Some(match_entry) = self.match_service.get_match(match_id) else {
                return Err(RematchError::MatchNotFound);
            };
            create_game_from_match(
                self.match_service.as_ref(),
                self.game_history_service.as_ref(),
                self.game_repository.as_ref(),
                self.game_service.as_ref(),
                &match_entry,
            );
        }
        Ok(())
    }
}
