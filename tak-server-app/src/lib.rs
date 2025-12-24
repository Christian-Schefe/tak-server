use std::sync::Arc;

use tokio::task::JoinHandle;

use crate::{
    domain::{
        account::{AdminAccountPolicy, ModeratorAccountPolicy},
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
        authentication::AuthenticationService, connection::PlayerConnectionPort, email::EmailPort,
        notification::ListenerNotificationPort,
    },
    processes::game_timeout_runner::GameTimeoutRunnerImpl,
    workflow::{
        account::{
            moderate::{ModeratePlayerUseCase, ModeratePlayerUseCaseImpl},
            register::{RegisterAccountUseCase, RegisterAccountUseCaseImpl},
        },
        chat::{
            message::{ChatMessageUseCase, ChatMessageUseCaseImpl},
            room::{ChatRoomUseCase, ChatRoomUseCaseImpl},
        },
        events::list::{ListEventsUseCase, ListEventsUseCaseImpl},
        gameplay::{
            do_action::{DoActionUseCase, DoActionUseCaseImpl},
            finalize_game::FinalizeGameWorkflowImpl,
            get::{GetOngoingGameUseCase, GetOngoingGameUseCaseImpl},
            list::{ListOngoingGameUseCase, ListOngoingGameUseCaseImpl},
            observe::{ObserveGameUseCase, ObserveGameUseCaseImpl},
            timeout::ObserveGameTimeoutUseCaseImpl,
        },
        matchmaking::{
            accept::{AcceptSeekUseCase, AcceptSeekUseCaseImpl},
            cancel::{CancelSeekUseCase, CancelSeekUseCaseImpl},
            cleanup::MatchCleanupJob,
            create::{CreateSeekUseCase, CreateSeekUseCaseImpl},
            get::{GetSeekUseCase, GetSeekUseCaseImpl},
            list::{ListSeeksUseCase, ListSeeksUseCaseImpl},
            rematch::{RematchUseCase, RematchUseCaseImpl},
        },
        player::set_online::{SetPlayerOnlineUseCase, SetPlayerOnlineUseCaseImpl},
    },
};

pub mod domain;
pub mod ports;
mod processes;
mod workflow;

pub struct Application {
    pub jobs: JoinHandle<()>,

    pub seek_accept_use_case: Box<dyn AcceptSeekUseCase>,
    pub seek_cancel_use_case: Box<dyn CancelSeekUseCase>,
    pub seek_create_use_case: Box<dyn CreateSeekUseCase>,
    pub seek_get_use_case: Box<dyn GetSeekUseCase>,
    pub seek_list_use_case: Box<dyn ListSeeksUseCase>,

    pub match_rematch_use_case: Box<dyn RematchUseCase>,

    pub player_set_online_use_case: Box<dyn SetPlayerOnlineUseCase>,

    pub game_do_action_use_case: Box<dyn DoActionUseCase>,
    pub game_get_ongoing_use_case: Box<dyn GetOngoingGameUseCase>,
    pub game_list_ongoing_use_case: Box<dyn ListOngoingGameUseCase>,
    pub game_observe_use_case: Box<dyn ObserveGameUseCase>,

    pub chat_message_use_case: Box<dyn ChatMessageUseCase>,
    pub chat_room_use_case: Box<dyn ChatRoomUseCase>,

    pub account_register_use_case: Box<dyn RegisterAccountUseCase>,
    pub account_ban_use_case: Box<dyn ModeratePlayerUseCase>,

    pub event_list_use_case: Box<dyn ListEventsUseCase>,
}

pub async fn build_core_application<
    L: ListenerNotificationPort + Send + Sync + 'static,
    C: PlayerConnectionPort + Send + Sync + 'static,
    G: GameRepository + Send + Sync + 'static,
    R: RatingRepository + Send + Sync + 'static,
    AS: AuthenticationService + Send + Sync + 'static,
    E: EmailPort + Send + Sync + 'static,
    ER: EventRepository + Send + Sync + 'static,
    PR: PlayerRepository + Send + Sync + 'static,
>(
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<C>,
    game_repository: Arc<G>,
    rating_repository: Arc<R>,
    authentication_service: Arc<AS>,
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
    let chat_content_policy = Arc::new(RustrictContentPolicy::new());
    let match_service = Arc::new(MatchServiceImpl::new());
    let ban_policy = Arc::new(AdminAccountPolicy);
    let silence_policy = Arc::new(ModeratorAccountPolicy);

    let finalize_game_workflow = Arc::new(FinalizeGameWorkflowImpl::new(
        game_repository.clone(),
        rating_service.clone(),
        rating_repository.clone(),
        game_history_service.clone(),
        match_service.clone(),
    ));
    let observe_game_timeout_use_case = Arc::new(ObserveGameTimeoutUseCaseImpl::new(
        game_service.clone(),
        finalize_game_workflow.clone(),
    ));
    let game_timeout_scheduler = Arc::new(GameTimeoutRunnerImpl::new(
        observe_game_timeout_use_case.clone(),
    ));

    let match_cleanup_job = MatchCleanupJob::new(match_service.clone());
    let jobs = tokio::spawn(async move {
        futures::join!(match_cleanup_job.run());
    });

    let application = Application {
        jobs,
        seek_accept_use_case: Box::new(AcceptSeekUseCaseImpl::new(
            seek_service.clone(),
            game_service.clone(),
            game_repository.clone(),
            game_history_service.clone(),
            match_service.clone(),
            listener_notification_port.clone(),
            game_timeout_scheduler.clone(),
        )),
        seek_cancel_use_case: Box::new(CancelSeekUseCaseImpl::new(
            seek_service.clone(),
            listener_notification_port.clone(),
        )),
        seek_create_use_case: Box::new(CreateSeekUseCaseImpl::new(
            seek_service.clone(),
            listener_notification_port.clone(),
        )),
        seek_get_use_case: Box::new(GetSeekUseCaseImpl::new(seek_service.clone())),
        seek_list_use_case: Box::new(ListSeeksUseCaseImpl::new(seek_service.clone())),

        match_rematch_use_case: Box::new(RematchUseCaseImpl::new(
            match_service.clone(),
            game_service.clone(),
            game_history_service.clone(),
            game_repository.clone(),
            game_timeout_scheduler.clone(),
        )),

        player_set_online_use_case: Box::new(SetPlayerOnlineUseCaseImpl::new(
            player_service.clone(),
            listener_notification_port.clone(),
        )),

        game_do_action_use_case: Box::new(DoActionUseCaseImpl::new(
            game_service.clone(),
            listener_notification_port.clone(),
            player_connection_port.clone(),
            finalize_game_workflow.clone(),
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
        chat_room_use_case: Box::new(ChatRoomUseCaseImpl::new(chat_room_service.clone())),

        account_register_use_case: Box::new(RegisterAccountUseCaseImpl::new(
            player_repository.clone(),
        )),
        account_ban_use_case: Box::new(ModeratePlayerUseCaseImpl::new(
            email_port.clone(),
            ban_policy.clone(),
            silence_policy.clone(),
            player_repository.clone(),
            authentication_service.clone(),
        )),

        event_list_use_case: Box::new(ListEventsUseCaseImpl::new(event_repository.clone())),
    };

    application
}
