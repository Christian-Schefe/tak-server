use std::sync::Arc;

use tak_core::{TakGameSettings, TakPlayer};

use crate::app::{
    domain::{
        PlayerId,
        seek::{SeekEvent, SeekService},
    },
    workflow::{event::EventDispatcher, matchmaking::GameType},
};

pub trait CreateSeekUseCase {
    fn create_seek(
        &self,
        player: PlayerId,
        opponent: Option<PlayerId>,
        color: Option<TakPlayer>,
        game_settings: TakGameSettings,
        game_type: GameType,
    ) -> Result<(), CreateSeekError>;
}

pub struct CreateSeekUseCaseImpl<S: SeekService, SD: EventDispatcher<SeekEvent>> {
    seek_service: Arc<S>,
    seek_event_dispatcher: Arc<SD>,
}

impl<S: SeekService, SD: EventDispatcher<SeekEvent>> CreateSeekUseCaseImpl<S, SD> {
    pub fn new(seek_service: Arc<S>, seek_event_dispatcher: Arc<SD>) -> Self {
        Self {
            seek_service,
            seek_event_dispatcher,
        }
    }
}

pub enum CreateSeekError {
    InvalidGameSettings,
    InvalidOpponent,
}

impl<S: SeekService, SD: EventDispatcher<SeekEvent>> CreateSeekUseCase
    for CreateSeekUseCaseImpl<S, SD>
{
    fn create_seek(
        &self,
        player: PlayerId,
        opponent: Option<PlayerId>,
        color: Option<TakPlayer>,
        game_settings: TakGameSettings,
        game_type: GameType,
    ) -> Result<(), CreateSeekError> {
        match self
            .seek_service
            .create_seek(player, opponent, color, game_settings, game_type)
        {
            Ok(_) => {}
            Err(crate::app::domain::seek::CreateSeekError::InvalidGameSettings) => {
                return Err(CreateSeekError::InvalidGameSettings);
            }
            Err(crate::app::domain::seek::CreateSeekError::InvalidOpponent) => {
                return Err(CreateSeekError::InvalidOpponent);
            }
        };

        let events = self.seek_service.take_events();
        self.seek_event_dispatcher.handle_events(events);

        Ok(())
    }
}
