use std::sync::Arc;

use tokio::task::JoinHandle;

use crate::{
    domain::{
        chat::{ChatRoomServiceImpl, RustrictContentPolicy},
        event::EventRepository,
        game::GameServiceImpl,
        game_history::{GameHistoryServiceImpl, GameRepository},
        r#match::MatchServiceImpl,
        moderation::{AdminAccountPolicy, HigherRoleAccountPolicy, ModeratorAccountPolicy},
        profile::AccountProfileRepository,
        rating::{RatingRepository, RatingServiceImpl},
        seek::SeekServiceImpl,
        spectator::SpectatorServiceImpl,
    },
    ports::{
        authentication::AuthenticationPort,
        connection::{AccountConnectionPort, AccountOnlineStatusPort},
        email::EmailPort,
        notification::ListenerNotificationPort,
        player_mapping::PlayerAccountMappingRepository,
    },
    processes::game_timeout_runner::GameTimeoutRunnerImpl,
    services::player_resolver::{PlayerResolverService, PlayerResolverServiceImpl},
    workflow::{
        account::{
            cleanup_guests::GuestCleanupJob,
            get_account::{GetAccountWorkflow, GetAccountWorkflowImpl},
            get_online::{GetOnlineAccountsUseCase, GetOnlineAccountsUseCaseImpl},
            get_profile::{GetProfileUseCase, GetProfileUseCaseImpl},
            get_snapshot::{GetSnapshotWorkflow, GetSnapshotWorkflowImpl},
            moderate::{ModeratePlayerUseCase, ModeratePlayerUseCaseImpl, ModerationPolicies},
            remove_account::RemoveAccountWorkflowImpl,
            set_online::{SetAccountOnlineUseCase, SetAccountOnlineUseCaseImpl},
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
        history::query::{GameHistoryQueryUseCase, GameHistoryQueryUseCaseImpl},
        matchmaking::{
            accept::{AcceptSeekUseCase, AcceptSeekUseCaseImpl},
            cancel::{CancelSeekUseCase, CancelSeekUseCaseImpl},
            cleanup::MatchCleanupJob,
            create::{CreateSeekUseCase, CreateSeekUseCaseImpl},
            create_game::CreateGameFromMatchWorkflowImpl,
            get::{GetSeekUseCase, GetSeekUseCaseImpl},
            list::{ListSeeksUseCase, ListSeeksUseCaseImpl},
            rematch::{RematchUseCase, RematchUseCaseImpl},
        },
        player::{
            get_rating::{PlayerGetRatingUseCase, PlayerGetRatingUseCaseImpl},
            notify_player::NotifyPlayerWorkflowImpl,
        },
    },
};

pub mod domain;
pub mod ports;
pub mod processes;
pub mod services;
pub mod workflow;

pub struct Application {
    pub jobs: JoinHandle<()>,

    pub seek_accept_use_case: Box<dyn AcceptSeekUseCase + Send + Sync + 'static>,
    pub seek_cancel_use_case: Box<dyn CancelSeekUseCase + Send + Sync + 'static>,
    pub seek_create_use_case: Box<dyn CreateSeekUseCase + Send + Sync + 'static>,
    pub seek_get_use_case: Box<dyn GetSeekUseCase + Send + Sync + 'static>,
    pub seek_list_use_case: Box<dyn ListSeeksUseCase + Send + Sync + 'static>,
    pub match_rematch_use_case: Box<dyn RematchUseCase + Send + Sync + 'static>,

    pub account_set_online_use_case: Box<dyn SetAccountOnlineUseCase + Send + Sync + 'static>,
    pub account_get_online_use_case: Box<dyn GetOnlineAccountsUseCase + Send + Sync + 'static>,

    pub player_get_rating_use_case: Box<dyn PlayerGetRatingUseCase + Send + Sync + 'static>,
    pub player_resolver_service: Arc<dyn PlayerResolverService + Send + Sync + 'static>,

    pub game_do_action_use_case: Box<dyn DoActionUseCase + Send + Sync + 'static>,
    pub game_get_ongoing_use_case: Box<dyn GetOngoingGameUseCase + Send + Sync + 'static>,
    pub game_list_ongoing_use_case: Box<dyn ListOngoingGameUseCase + Send + Sync + 'static>,
    pub game_observe_use_case: Box<dyn ObserveGameUseCase + Send + Sync + 'static>,

    pub game_history_query_use_case: Box<dyn GameHistoryQueryUseCase + Send + Sync + 'static>,

    pub chat_message_use_case: Box<dyn ChatMessageUseCase + Send + Sync + 'static>,
    pub chat_room_use_case: Box<dyn ChatRoomUseCase + Send + Sync + 'static>,

    pub account_moderate_use_case: Box<dyn ModeratePlayerUseCase + Send + Sync + 'static>,

    pub event_list_use_case: Box<dyn ListEventsUseCase + Send + Sync + 'static>,

    pub get_snapshot_workflow: Arc<dyn GetSnapshotWorkflow + Send + Sync + 'static>,
    pub get_account_workflow: Arc<dyn GetAccountWorkflow + Send + Sync + 'static>,
    pub get_profile_use_case: Arc<dyn GetProfileUseCase + Send + Sync + 'static>,
}

pub async fn build_application<
    L: ListenerNotificationPort + Send + Sync + 'static,
    C: AccountConnectionPort + Send + Sync + 'static,
    G: GameRepository + Send + Sync + 'static,
    R: RatingRepository + Send + Sync + 'static,
    AS: AuthenticationPort + Send + Sync + 'static,
    E: EmailPort + Send + Sync + 'static,
    ER: EventRepository + Send + Sync + 'static,
    PR: PlayerAccountMappingRepository + Send + Sync + 'static,
    PF: AccountProfileRepository + Send + Sync + 'static,
    AC: AccountOnlineStatusPort + Send + Sync + 'static,
>(
    game_repository: Arc<G>,
    player_repository: Arc<PR>,
    rating_repository: Arc<R>,
    event_repository: Arc<ER>,
    email_port: Arc<E>,
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<C>,
    authentication_service: Arc<AS>,
    profile_repository: Arc<PF>,
    account_online_status_port: Arc<AC>,
) -> Application {
    let seek_service = Arc::new(SeekServiceImpl::new());
    let game_service = Arc::new(GameServiceImpl::new());
    let spectator_service = Arc::new(SpectatorServiceImpl::new());
    let chat_room_service = Arc::new(ChatRoomServiceImpl::new());
    let game_history_service = Arc::new(GameHistoryServiceImpl::new());
    let rating_service = Arc::new(RatingServiceImpl::new());
    let chat_content_policy = Arc::new(RustrictContentPolicy::new());
    let match_service = Arc::new(MatchServiceImpl::new());

    let policies = ModerationPolicies {
        ban_policy: Arc::new(AdminAccountPolicy),
        kick_policy: Arc::new(ModeratorAccountPolicy),
        silence_policy: Arc::new(ModeratorAccountPolicy),
        set_moderator_policy: Arc::new(HigherRoleAccountPolicy),
        set_admin_policy: Arc::new(HigherRoleAccountPolicy),
        set_user_policy: Arc::new(HigherRoleAccountPolicy),
    };

    let player_resolver_service =
        Arc::new(PlayerResolverServiceImpl::new(player_repository.clone()));

    let get_account_workflow = Arc::new(GetAccountWorkflowImpl::new(
        authentication_service.clone(),
        player_repository.clone(),
    ));

    let get_snapshot_workflow = Arc::new(GetSnapshotWorkflowImpl::new(
        get_account_workflow.clone(),
        rating_repository.clone(),
        rating_service.clone(),
    ));

    let notify_player_workflow = Arc::new(NotifyPlayerWorkflowImpl::new(
        listener_notification_port.clone(),
        player_connection_port.clone(),
        game_service.clone(),
        spectator_service.clone(),
        player_resolver_service.clone(),
    ));

    let finalize_game_workflow = Arc::new(FinalizeGameWorkflowImpl::new(
        game_repository.clone(),
        rating_service.clone(),
        rating_repository.clone(),
        game_history_service.clone(),
        match_service.clone(),
        notify_player_workflow.clone(),
        spectator_service.clone(),
        listener_notification_port.clone(),
        get_account_workflow.clone(),
    ));
    let observe_game_timeout_use_case = Arc::new(ObserveGameTimeoutUseCaseImpl::new(
        game_service.clone(),
        finalize_game_workflow.clone(),
    ));
    let game_timeout_scheduler = Arc::new(GameTimeoutRunnerImpl::new(
        observe_game_timeout_use_case.clone(),
    ));

    let create_game_from_match_workflow = Arc::new(CreateGameFromMatchWorkflowImpl::new(
        match_service.clone(),
        game_history_service.clone(),
        game_repository.clone(),
        game_service.clone(),
        game_timeout_scheduler.clone(),
        listener_notification_port.clone(),
        get_snapshot_workflow.clone(),
    ));

    let remove_account_workflow =
        Arc::new(RemoveAccountWorkflowImpl::new(player_repository.clone()));

    let match_cleanup_job = MatchCleanupJob::new(match_service.clone());
    let guest_cleanup_job = GuestCleanupJob::new(
        authentication_service.clone(),
        remove_account_workflow.clone(),
    );

    let jobs = tokio::spawn(async move {
        futures::join!(match_cleanup_job.run(), guest_cleanup_job.run());
    });

    let application = Application {
        jobs,
        seek_accept_use_case: Box::new(AcceptSeekUseCaseImpl::new(
            seek_service.clone(),
            match_service.clone(),
            listener_notification_port.clone(),
            create_game_from_match_workflow.clone(),
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
            create_game_from_match_workflow.clone(),
        )),

        account_set_online_use_case: Box::new(SetAccountOnlineUseCaseImpl::new(
            account_online_status_port.clone(),
            listener_notification_port.clone(),
        )),
        account_get_online_use_case: Box::new(GetOnlineAccountsUseCaseImpl::new(
            account_online_status_port.clone(),
        )),

        player_get_rating_use_case: Box::new(PlayerGetRatingUseCaseImpl::new(
            rating_repository.clone(),
            rating_service.clone(),
        )),
        player_resolver_service,

        game_do_action_use_case: Box::new(DoActionUseCaseImpl::new(
            game_service.clone(),
            notify_player_workflow.clone(),
            finalize_game_workflow.clone(),
        )),
        game_get_ongoing_use_case: Box::new(GetOngoingGameUseCaseImpl::new(game_service.clone())),
        game_list_ongoing_use_case: Box::new(ListOngoingGameUseCaseImpl::new(game_service.clone())),
        game_observe_use_case: Box::new(ObserveGameUseCaseImpl::new(
            game_service.clone(),
            spectator_service.clone(),
        )),

        game_history_query_use_case: Box::new(GameHistoryQueryUseCaseImpl::new(
            game_repository.clone(),
        )),

        chat_message_use_case: Box::new(ChatMessageUseCaseImpl::new(
            listener_notification_port.clone(),
            player_connection_port.clone(),
            chat_room_service.clone(),
            chat_content_policy.clone(),
        )),
        chat_room_use_case: Box::new(ChatRoomUseCaseImpl::new(chat_room_service.clone())),

        account_moderate_use_case: Box::new(ModeratePlayerUseCaseImpl::new(
            email_port.clone(),
            policies,
            player_repository.clone(),
            authentication_service.clone(),
        )),

        event_list_use_case: Box::new(ListEventsUseCaseImpl::new(event_repository.clone())),

        get_snapshot_workflow,
        get_account_workflow,
        get_profile_use_case: Arc::new(GetProfileUseCaseImpl::new(
            player_repository.clone(),
            profile_repository.clone(),
        )),
    };

    application
}
