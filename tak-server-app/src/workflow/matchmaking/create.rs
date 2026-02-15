use std::sync::Arc;

use tak_core::{TakGameSettings, TakPlayer};

use crate::{
    domain::{
        PlayerId,
        seek::{CreateSeekError, SeekService},
    },
    ports::notification::{ListenerMessage, ListenerNotificationPort},
    workflow::matchmaking::SeekView,
};

pub trait CreateSeekUseCase {
    fn create_seek(
        &self,
        player: PlayerId,
        opponent: Option<PlayerId>,
        color: Option<TakPlayer>,
        game_settings: TakGameSettings,
        is_rated: bool,
    ) -> Result<SeekView, CreateSeekError>;
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
    ) -> Result<SeekView, CreateSeekError> {
        let created_seek =
            self.seek_service
                .create_seek(player, opponent, color, game_settings, is_rated)?;
        let seek_view: SeekView = created_seek.into();
        let message = ListenerMessage::SeekCreated {
            seek: seek_view.clone(),
        };

        self.notification_port.notify_all(&message);

        Ok(seek_view)
    }
}
