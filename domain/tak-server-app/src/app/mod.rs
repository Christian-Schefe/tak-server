use std::sync::Arc;

use crate::{
    app::{
        event::{EventDispatcher, InMemoryEventDispatcher},
        gameplay::{
            do_action::{DoActionUseCase, DoActionUseCaseImpl},
            get::{GetOngoingGameUseCase, GetOngoingGameUseCaseImpl},
            list::{ListOngoingGameUseCase, ListOngoingGameUseCaseImpl},
        },
        matchmaking::{
            accept::{AcceptSeekUseCase, AcceptSeekUseCaseImpl},
            cancel::{CancelSeekUseCase, CancelSeekUseCaseImpl},
            create::{CreateSeekUseCase, CreateSeekUseCaseImpl},
            get::{GetSeekUseCase, GetSeekUseCaseImpl},
            list::{ListSeeksUseCase, ListSeeksUseCaseImpl},
            notify_seek::SeekEventNotifier,
        },
        player::{
            notify_online::PlayerEventNotifier,
            set_online::{SetPlayerOnlineUseCase, SetPlayerOnlineUseCaseImpl},
        },
        ports::{connection::PlayerConnectionPort, notification::ListenerNotificationPort},
    },
    domain::{
        game::{GameRepository, GameServiceImpl},
        player::PlayerServiceImpl,
        seek::SeekServiceImpl,
        spectator::SpectatorServiceImpl,
    },
};

mod event;
pub mod gameplay;
pub mod matchmaking;
pub mod player;
mod ports;

pub struct Application {
    pub seek_accept_use_case: Box<dyn AcceptSeekUseCase>,
    pub seek_cancel_use_case: Box<dyn CancelSeekUseCase>,
    pub seek_create_use_case: Box<dyn CreateSeekUseCase>,
    pub seek_get_use_case: Box<dyn GetSeekUseCase>,
    pub seek_list_use_case: Box<dyn ListSeeksUseCase>,

    pub player_set_online_use_case: Box<dyn SetPlayerOnlineUseCase>,

    pub game_do_action_use_case: Box<dyn DoActionUseCase>,
    pub game_get_ongoing_use_case: Box<dyn GetOngoingGameUseCase>,
    pub game_list_ongoing_use_case: Box<dyn ListOngoingGameUseCase>,
}

pub fn build_application<
    L: ListenerNotificationPort + 'static,
    C: PlayerConnectionPort + 'static,
    G: GameRepository + 'static,
>(
    listener_notification_port: L,
    player_connection_port: C,
    game_repository: G,
) -> Application {
    let game_repository = Arc::new(game_repository);
    let listener_notification_port = Arc::new(listener_notification_port);
    let player_connection_port = Arc::new(player_connection_port);

    let seek_service = Arc::new(SeekServiceImpl::new());
    let game_service = Arc::new(GameServiceImpl::new(game_repository.clone()));
    let player_service = Arc::new(PlayerServiceImpl::new());
    let spectator_service = Arc::new(SpectatorServiceImpl::new());

    let mut seek_event_dispatcher = InMemoryEventDispatcher::new();
    seek_event_dispatcher.register_listener(Box::new(SeekEventNotifier::new(
        listener_notification_port.clone(),
    )));

    let mut player_event_dispatcher = InMemoryEventDispatcher::new();
    player_event_dispatcher.register_listener(Box::new(PlayerEventNotifier::new(
        listener_notification_port.clone(),
    )));

    let game_event_dispatcher = InMemoryEventDispatcher::new();

    let seek_event_dispatcher = Arc::new(seek_event_dispatcher);
    let player_event_dispatcher = Arc::new(player_event_dispatcher);
    let game_event_dispatcher = Arc::new(game_event_dispatcher);

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

        player_set_online_use_case: Box::new(SetPlayerOnlineUseCaseImpl::new(
            player_service.clone(),
            player_event_dispatcher.clone(),
            spectator_service.clone(),
        )),

        game_do_action_use_case: Box::new(DoActionUseCaseImpl::new(
            game_service.clone(),
            listener_notification_port.clone(),
            player_connection_port.clone(),
            game_event_dispatcher.clone(),
        )),
        game_get_ongoing_use_case: Box::new(GetOngoingGameUseCaseImpl::new(game_service.clone())),
        game_list_ongoing_use_case: Box::new(ListOngoingGameUseCaseImpl::new(game_service.clone())),
    };

    application
}
