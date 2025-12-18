use std::sync::Arc;

use crate::{
    app::{
        matchmaking::{
            accept::{AcceptSeekUseCase, AcceptSeekUseCaseImpl},
            cancel::{CancelSeekUseCase, CancelSeekUseCaseImpl},
            create::{CreateSeekUseCase, CreateSeekUseCaseImpl},
            event::{InMemorySeekEventDispatcher, SeekEventDispatcher},
            get::{GetSeekUseCase, GetSeekUseCaseImpl},
            list::{ListSeeksUseCase, ListSeeksUseCaseImpl},
            notify::SeekEventNotifier,
        },
        ports::notification::ListenerNotificationPort,
    },
    domain::{
        game::{GameRepository, GameServiceImpl},
        seek::SeekServiceImpl,
    },
};

pub mod gameplay;
pub mod matchmaking;
mod ports;

pub struct Application {
    pub seek_accept_use_case: Box<dyn AcceptSeekUseCase>,
    pub seek_cancel_use_case: Box<dyn CancelSeekUseCase>,
    pub seek_create_use_case: Box<dyn CreateSeekUseCase>,
    pub seek_get_use_case: Box<dyn GetSeekUseCase>,
    pub seek_list_use_case: Box<dyn ListSeeksUseCase>,
}

pub fn build_application<L: ListenerNotificationPort + 'static, G: GameRepository + 'static>(
    listener_notification_port: L,
    game_repository: G,
) -> Application {
    let game_repository = Arc::new(game_repository);
    let listener_notification_port = Arc::new(listener_notification_port);

    let seek_service = Arc::new(SeekServiceImpl::new());
    let game_service = Arc::new(GameServiceImpl::new(game_repository.clone()));

    let mut seek_event_dispatcher = InMemorySeekEventDispatcher::new();
    seek_event_dispatcher.register_listener(Box::new(SeekEventNotifier::new(
        listener_notification_port.clone(),
    )));

    let seek_event_dispatcher = Arc::new(seek_event_dispatcher);

    let application = Application {
        seek_accept_use_case: Box::new(AcceptSeekUseCaseImpl::new(
            seek_service.clone(),
            seek_event_dispatcher.clone(),
            game_service.clone(),
        )),
        seek_cancel_use_case: Box::new(CancelSeekUseCaseImpl::new(
            seek_service.clone(),
            seek_event_dispatcher.clone(),
        )),
        seek_create_use_case: Box::new(CreateSeekUseCaseImpl::new(
            seek_service.clone(),
            seek_event_dispatcher.clone(),
        )),
        seek_get_use_case: Box::new(GetSeekUseCaseImpl::new(seek_service.clone())),
        seek_list_use_case: Box::new(ListSeeksUseCaseImpl::new(seek_service.clone())),
    };

    application
}
