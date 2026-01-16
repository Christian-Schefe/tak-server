use std::sync::Arc;

use tak_core::{TakGameSettings, TakPlayer};

use crate::{
    domain::{
        PlayerId,
        seek::{CreateSeekError, SeekService},
    },
    ports::notification::{ListenerMessage, ListenerNotificationPort},
};

pub trait CreateSeekUseCase {
    fn create_seek(
        &self,
        player: PlayerId,
        opponent: Option<PlayerId>,
        color: Option<TakPlayer>,
        game_settings: TakGameSettings,
        is_rated: bool,
    ) -> Result<(), CreateSeekError>;
}

pub struct CreateSeekUseCaseImpl<S: SeekService, L: ListenerNotificationPort> {
    seek_service: Arc<S>,
    notification_port: Arc<L>,
}

impl<S: SeekService, L: ListenerNotificationPort> CreateSeekUseCaseImpl<S, L> {
    pub fn new(seek_service: Arc<S>, notification_port: Arc<L>) -> Self {
        Self {
            seek_service,
            notification_port,
        }
    }
}

impl<S: SeekService + Send + Sync + 'static, L: ListenerNotificationPort + Send + Sync + 'static>
    CreateSeekUseCase for CreateSeekUseCaseImpl<S, L>
{
    fn create_seek(
        &self,
        player: PlayerId,
        opponent: Option<PlayerId>,
        color: Option<TakPlayer>,
        game_settings: TakGameSettings,
        is_rated: bool,
    ) -> Result<(), CreateSeekError> {
        let created_seek =
            self.seek_service
                .create_seek(player, opponent, color, game_settings, is_rated)?;
        let message = ListenerMessage::SeekCreated {
            seek: created_seek.into(),
        };

        self.notification_port.notify_all(message);

        Ok(())
    }
}
