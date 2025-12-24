use std::sync::Arc;

use tokio::task::JoinHandle;

use crate::app::{
    domain::{
        account::{
            AccountFactoryImpl, AccountRepository, AdminAccountPolicy, ModeratorAccountPolicy,
        },
        chat::{ChatRoomServiceImpl, RustrictContentPolicy},
        event::EventRepository,
        game::GameServiceImpl,
        game_history::{GameHistoryServiceImpl, GameRepository},
        r#match::MatchServiceImpl,
        player::{PlayerRepository, PlayerServiceImpl},
        rating::{RatingRepository, RatingServiceImpl},
        seek::SeekServiceImpl,
        spectator::SpectatorServiceImpl,
    },
    ports::{
        authentication::AuthenticationService, connection::PlayerConnectionPort,
        contact::ContactRepository, email::EmailPort, notification::ListenerNotificationPort,
    },
    workflow::{
        account::{
            login::{LoginAccountUseCase, LoginAccountUseCaseImpl},
            moderate::{ModeratePlayerUseCase, ModeratePlayerUseCaseImpl},
            register::{RegisterAccountUseCase, RegisterAccountUseCaseImpl},
        },
        chat::message::{ChatMessageUseCase, ChatMessageUseCaseImpl},
        event::{EventDispatcher, InMemoryEventDispatcher},
        events::list::{ListEventsUseCase, ListEventsUseCaseImpl},
        gameplay::{
            do_action::{DoActionUseCase, DoActionUseCaseImpl},
            finalize_game::FinalizeGameListener,
            get::{GetOngoingGameUseCase, GetOngoingGameUseCaseImpl},
            list::{ListOngoingGameUseCase, ListOngoingGameUseCaseImpl},
            observe::{ObserveGameUseCase, ObserveGameUseCaseImpl},
        },
        matchmaking::{
            accept::{AcceptSeekUseCase, AcceptSeekUseCaseImpl},
            cancel::{CancelSeekUseCase, CancelSeekUseCaseImpl},
            cleanup::MatchCleanupJob,
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
};

pub mod account;
pub mod chat;
mod event;
pub mod events;
pub mod gameplay;
pub mod matchmaking;
pub mod player;

pub struct Application {
    pub jobs: JoinHandle<()>,

    pub seek_accept_use_case: Box<dyn AcceptSeekUseCase>,
    pub seek_cancel_use_case: Box<dyn CancelSeekUseCase>,
    pub seek_create_use_case: Box<dyn CreateSeekUseCase>,
    pub seek_get_use_case: Box<dyn GetSeekUseCase>,
    pub seek_list_use_case: Box<dyn ListSeeksUseCase>,

    pub player_set_online_use_case: Box<dyn SetPlayerOnlineUseCase>,

    pub game_do_action_use_case: Box<dyn DoActionUseCase>,
    pub game_get_ongoing_use_case: Box<dyn GetOngoingGameUseCase>,
    pub game_list_ongoing_use_case: Box<dyn ListOngoingGameUseCase>,
    pub game_observe_use_case: Box<dyn ObserveGameUseCase>,

    pub chat_message_use_case: Box<dyn ChatMessageUseCase>,

    pub account_register_use_case: Box<dyn RegisterAccountUseCase>,
    pub account_login_use_case: Box<dyn LoginAccountUseCase>,
    pub account_ban_use_case: Box<dyn ModeratePlayerUseCase>,

    pub event_list_use_case: Box<dyn ListEventsUseCase>,
}

pub async fn build_application<
    L: ListenerNotificationPort + 'static,
    C: PlayerConnectionPort + 'static,
    G: GameRepository + 'static,
    A: AccountRepository + 'static,
    R: RatingRepository + 'static,
    AS: AuthenticationService + 'static,
    CR: ContactRepository + 'static,
    E: EmailPort + 'static,
    ER: EventRepository + 'static,
    PR: PlayerRepository + 'static,
>(
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<C>,
    game_repository: Arc<G>,
    account_repository: Arc<A>,
    rating_repository: Arc<R>,
    authentication_service: Arc<AS>,
    contact_repository: Arc<CR>,
    email_port: Arc<E>,
    event_repository: Arc<ER>,
    player_repository: Arc<PR>,
) -> Application {
    let seek_service = Arc::new(SeekServiceImpl::new());
    let game_service = Arc::new(GameServiceImpl::new());
    let player_service = Arc::new(PlayerServiceImpl::new());
    let spectator_service = Arc::new(SpectatorServiceImpl::new());
    let chat_room_service = Arc::new(ChatRoomServiceImpl::new());
    let game_history_service = Arc::new(GameHistoryServiceImpl::new());
    let rating_service = Arc::new(RatingServiceImpl::new());
    let account_factory = Arc::new(AccountFactoryImpl::new());
    let chat_content_policy = Arc::new(RustrictContentPolicy::new());
    let match_service = Arc::new(MatchServiceImpl::new());
    let ban_policy = Arc::new(AdminAccountPolicy);
    let silence_policy = Arc::new(ModeratorAccountPolicy);

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
        match_service.clone(),
    )));

    let seek_event_dispatcher = Arc::new(seek_event_dispatcher);
    let player_event_dispatcher = Arc::new(player_event_dispatcher);
    let game_event_dispatcher = Arc::new(game_event_dispatcher);

    let match_cleanup_job = MatchCleanupJob::new(match_service.clone());
    let jobs = tokio::spawn(async move {
        futures::join!(match_cleanup_job.run());
    });

    let application = Application {
        jobs,
        seek_accept_use_case: Box::new(AcceptSeekUseCaseImpl::new(
            seek_service.clone(),
            seek_event_dispatcher.clone(),
            game_service.clone(),
            game_repository.clone(),
            game_history_service.clone(),
            match_service.clone(),
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
        )),

        game_do_action_use_case: Box::new(DoActionUseCaseImpl::new(
            game_service.clone(),
            listener_notification_port.clone(),
            player_connection_port.clone(),
            game_event_dispatcher.clone(),
        )),
        game_get_ongoing_use_case: Box::new(GetOngoingGameUseCaseImpl::new(game_service.clone())),
        game_list_ongoing_use_case: Box::new(ListOngoingGameUseCaseImpl::new(game_service.clone())),
        game_observe_use_case: Box::new(ObserveGameUseCaseImpl::new(
            game_service.clone(),
            spectator_service.clone(),
        )),

        chat_message_use_case: Box::new(ChatMessageUseCaseImpl::new(
            listener_notification_port.clone(),
            player_connection_port.clone(),
            chat_room_service.clone(),
            chat_content_policy.clone(),
            player_repository.clone(),
        )),

        account_register_use_case: Box::new(RegisterAccountUseCaseImpl::new(
            account_factory.clone(),
            account_repository.clone(),
            authentication_service.clone(),
            contact_repository.clone(),
            player_repository.clone(),
        )),
        account_login_use_case: Box::new(LoginAccountUseCaseImpl::new(
            authentication_service.clone(),
            player_repository.clone(),
        )),
        account_ban_use_case: Box::new(ModeratePlayerUseCaseImpl::new(
            account_repository.clone(),
            contact_repository.clone(),
            email_port.clone(),
            ban_policy.clone(),
            silence_policy.clone(),
            player_repository.clone(),
        )),

        event_list_use_case: Box::new(ListEventsUseCaseImpl::new(event_repository.clone())),
    };

    application
}
