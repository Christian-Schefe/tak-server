use std::sync::Arc;

use crate::{
    app::{
        chat::message::{ChatMessageUseCase, ChatMessageUseCaseImpl},
        event::{EventDispatcher, InMemoryEventDispatcher},
        gameplay::{
            do_action::{DoActionUseCase, DoActionUseCaseImpl},
            finalize_game::FinalizeGameListener,
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
    },
    domain::{
        account::AccountRepository,
        chat::{ChatRoomServiceImpl, RustrictContentPolicy},
        game::GameServiceImpl,
        game_history::{GameHistoryServiceImpl, GameRepository},
        player::PlayerServiceImpl,
        rating::{RatingRepository, RatingServiceImpl},
        seek::SeekServiceImpl,
        spectator::SpectatorServiceImpl,
    },
    ports::{connection::PlayerConnectionPort, notification::ListenerNotificationPort},
};

pub mod chat;
mod event;
pub mod gameplay;
pub mod matchmaking;
pub mod player;

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

    pub chat_message_use_case: Box<dyn ChatMessageUseCase>,
}

pub fn build_application<
    L: ListenerNotificationPort + 'static,
    C: PlayerConnectionPort + 'static,
    G: GameRepository + 'static,
    A: AccountRepository + 'static,
    R: RatingRepository + 'static,
>(
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<C>,
    game_repository: Arc<G>,
    account_repository: Arc<A>,
    rating_repository: Arc<R>,
) -> Application {
    let seek_service = Arc::new(SeekServiceImpl::new());
    let game_service = Arc::new(GameServiceImpl::new());
    let player_service = Arc::new(PlayerServiceImpl::new());
    let spectator_service = Arc::new(SpectatorServiceImpl::new());
    let chat_room_service = Arc::new(ChatRoomServiceImpl::new());
    let game_history_service = Arc::new(GameHistoryServiceImpl::new());
    let rating_service = Arc::new(RatingServiceImpl::new());

    let chat_content_policy = Arc::new(RustrictContentPolicy::new());

    let mut seek_event_dispatcher = InMemoryEventDispatcher::new();
    seek_event_dispatcher.register_listener(Box::new(SeekEventNotifier::new(
        listener_notification_port.clone(),
    )));

    let mut player_event_dispatcher = InMemoryEventDispatcher::new();
    player_event_dispatcher.register_listener(Box::new(PlayerEventNotifier::new(
        listener_notification_port.clone(),
    )));

    let mut game_event_dispatcher = InMemoryEventDispatcher::new();
    game_event_dispatcher.register_listener(Box::new(FinalizeGameListener::new(
        game_repository.clone(),
        rating_service.clone(),
        rating_repository.clone(),
        game_history_service.clone(),
    )));

    let seek_event_dispatcher = Arc::new(seek_event_dispatcher);
    let player_event_dispatcher = Arc::new(player_event_dispatcher);
    let game_event_dispatcher = Arc::new(game_event_dispatcher);

    let application = Application {
        seek_accept_use_case: Box::new(AcceptSeekUseCaseImpl::new(
            seek_service.clone(),
            seek_event_dispatcher.clone(),
            game_service.clone(),
            game_repository.clone(),
            game_history_service.clone(),
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
        chat_message_use_case: Box::new(ChatMessageUseCaseImpl::new(
            listener_notification_port.clone(),
            player_connection_port.clone(),
            chat_room_service.clone(),
            chat_content_policy.clone(),
            account_repository.clone(),
        )),
    };

    application
}
