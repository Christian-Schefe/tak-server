use std::sync::Arc;

use crate::{
    domain::{
        chat::{ChatRepository, ChatRoomServiceImpl, RustrictContentPolicy},
        event::EventRepository,
        game::GameServiceImpl,
        game_history::{GameHistoryServiceImpl, GameRepository},
        matches::{MatchReadinessServiceImpl, MatchRepository},
        moderation::{AdminAccountPolicy, HigherRoleAccountPolicy, ModeratorAccountPolicy},
        profile::{AccountProfileRepository, ProfilePictureRepository},
        puzzle::PuzzleRepository,
        rating::{RatingRepository, RatingServiceImpl},
        seek::SeekServiceImpl,
        spectator::SpectatorServiceImpl,
        stats::{RatingHistoryRepository, StatsRepository},
        tournament::{TournamentPlayerRepository, TournamentRepository, TournamentRoundRepository},
    },
    ports::{
        authentication::AuthenticationPort,
        connection::{AccountConnectionPort, AccountOnlineStatusPort},
        email::EmailPort,
        notification::ListenerNotificationPort,
        player_mapping::PlayerAccountMappingRepository,
    },
    processes::{
        disconnect_timeout_runner::DisconnectTimeoutRunnerImpl,
        game_timeout_runner::GameTimeoutRunnerImpl,
    },
    services::player_resolver::{PlayerResolverService, PlayerResolverServiceImpl},
    workflow::{
        account::{
            get_account::{GetAccountWorkflow, GetAccountWorkflowImpl},
            get_online::{GetOnlineAccountsUseCase, GetOnlineAccountsUseCaseImpl},
            get_profile::{GetProfileUseCase, GetProfileUseCaseImpl},
            get_snapshot::{GetSnapshotWorkflow, GetSnapshotWorkflowImpl},
            moderate::{ModeratePlayerUseCase, ModeratePlayerUseCaseImpl, ModerationPolicies},
            set_online::{SetAccountOnlineUseCase, SetAccountOnlineUseCaseImpl},
            update_profile::{UpdateProfileUseCase, UpdateProfileUseCaseImpl},
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
        listener::disconnect::{ListenerDisconnectUseCase, ListenerDisconnectUseCaseImpl},
        matchmaking::{
            accept::{AcceptSeekUseCase, AcceptSeekUseCaseImpl},
            cancel::{CancelSeekUseCase, CancelSeekUseCaseImpl},
            create::{CreateSeekUseCase, CreateSeekUseCaseImpl},
            create_game::CreateGameFromMatchWorkflowImpl,
            get::{GetMatchUseCase, GetMatchUseCaseImpl, GetSeekUseCase, GetSeekUseCaseImpl},
            list::{ListSeeksUseCase, ListSeeksUseCaseImpl},
            readiness::{MatchReadinessUseCase, MatchReadinessUseCaseImpl},
        },
        player::{
            get_rating::{PlayerGetRatingUseCase, PlayerGetRatingUseCaseImpl},
            get_stats::{GetPlayerStatsUseCase, GetPlayerStatsUseCaseImpl},
            notify_player::NotifyPlayerWorkflowImpl,
        },
        puzzle::{
            get::{GetPuzzleUseCase, GetPuzzleUseCaseImpl},
            solve::{SolvePuzzleUseCase, SolvePuzzleUseCaseImpl},
        },
        shutdown::{ShutdownWorkflow, ShutdownWorkflowImpl},
        tournament::{
            get::{GetTournamentUseCase, GetTournamentUseCaseImpl},
            host::{HostTournamentUseCase, HostTournamentUseCaseImpl},
            register::{
                TournamentPlayerRegistrationUseCase, TournamentPlayerRegistrationUseCaseImpl,
            },
            tournament_match::TournamentMatchWorkflowImpl,
        },
    },
};

pub mod domain;
pub mod ports;
pub mod processes;
pub mod services;
pub mod workflow;

pub struct Application {
    pub seek_accept_use_case: Arc<dyn AcceptSeekUseCase + Send + Sync + 'static>,
    pub seek_cancel_use_case: Arc<dyn CancelSeekUseCase + Send + Sync + 'static>,
    pub seek_create_use_case: Arc<dyn CreateSeekUseCase + Send + Sync + 'static>,
    pub seek_get_use_case: Arc<dyn GetSeekUseCase + Send + Sync + 'static>,
    pub seek_list_use_case: Arc<dyn ListSeeksUseCase + Send + Sync + 'static>,
    pub match_readiness_use_case: Arc<dyn MatchReadinessUseCase + Send + Sync + 'static>,

    pub account_set_online_use_case: Arc<dyn SetAccountOnlineUseCase + Send + Sync + 'static>,
    pub account_get_online_use_case: Arc<dyn GetOnlineAccountsUseCase + Send + Sync + 'static>,

    pub listener_disconnect_use_case: Arc<dyn ListenerDisconnectUseCase + Send + Sync + 'static>,

    pub player_get_rating_use_case: Arc<dyn PlayerGetRatingUseCase + Send + Sync + 'static>,
    pub player_resolver_service: Arc<dyn PlayerResolverService + Send + Sync + 'static>,

    pub game_do_action_use_case: Arc<dyn DoActionUseCase + Send + Sync + 'static>,
    pub game_get_ongoing_use_case: Arc<dyn GetOngoingGameUseCase + Send + Sync + 'static>,
    pub game_list_ongoing_use_case: Arc<dyn ListOngoingGameUseCase + Send + Sync + 'static>,
    pub game_observe_use_case: Arc<dyn ObserveGameUseCase + Send + Sync + 'static>,

    pub game_history_query_use_case: Arc<dyn GameHistoryQueryUseCase + Send + Sync + 'static>,

    pub chat_message_use_case: Arc<dyn ChatMessageUseCase + Send + Sync + 'static>,
    pub chat_room_use_case: Arc<dyn ChatRoomUseCase + Send + Sync + 'static>,

    pub account_moderate_use_case: Arc<dyn ModeratePlayerUseCase + Send + Sync + 'static>,

    pub event_list_use_case: Arc<dyn ListEventsUseCase + Send + Sync + 'static>,

    pub get_snapshot_workflow: Arc<dyn GetSnapshotWorkflow + Send + Sync + 'static>,
    pub get_account_workflow: Arc<dyn GetAccountWorkflow + Send + Sync + 'static>,
    pub get_profile_use_case: Arc<dyn GetProfileUseCase + Send + Sync + 'static>,
    pub update_profile_use_case: Arc<dyn UpdateProfileUseCase + Send + Sync + 'static>,

    pub get_stats_use_case: Arc<dyn GetPlayerStatsUseCase + Send + Sync + 'static>,

    pub get_puzzle_use_case: Arc<dyn GetPuzzleUseCase + Send + Sync + 'static>,
    pub solve_puzzle_use_case: Arc<dyn SolvePuzzleUseCase + Send + Sync + 'static>,

    pub get_tournaments_use_case: Arc<dyn GetTournamentUseCase + Send + Sync + 'static>,
    pub tournament_player_registration_use_case:
        Arc<dyn TournamentPlayerRegistrationUseCase + Send + Sync + 'static>,
    pub host_tournament_use_case: Arc<dyn HostTournamentUseCase + Send + Sync + 'static>,

    pub match_get_use_case: Arc<dyn GetMatchUseCase + Send + Sync + 'static>,

    pub shutdown_workflow: Arc<dyn ShutdownWorkflow + Send + Sync + 'static>,
}

pub async fn build_application<
    L: ListenerNotificationPort + Send + Sync + 'static,
    C: AccountConnectionPort + Send + Sync + 'static,
    G: GameRepository + Send + Sync + 'static,
    R: RatingRepository + Send + Sync + 'static,
    S: StatsRepository + Send + Sync + 'static,
    AS: AuthenticationPort + Send + Sync + 'static,
    E: EmailPort + Send + Sync + 'static,
    ER: EventRepository + Send + Sync + 'static,
    PR: PlayerAccountMappingRepository + Send + Sync + 'static,
    PF: AccountProfileRepository + Send + Sync + 'static,
    AC: AccountOnlineStatusPort + Send + Sync + 'static,
    PFP: ProfilePictureRepository + Send + Sync + 'static,
    PZR: PuzzleRepository + Send + Sync + 'static,
    CR: ChatRepository + Send + Sync + 'static,
    RH: RatingHistoryRepository + Send + Sync + 'static,
    TR: TournamentRepository + Send + Sync + 'static,
    TPR: TournamentPlayerRepository + Send + Sync + 'static,
    MR: MatchRepository + Send + Sync + 'static,
    TRR: TournamentRoundRepository + Send + Sync + 'static,
>(
    game_repository: Arc<G>,
    player_repository: Arc<PR>,
    rating_repository: Arc<R>,
    event_repository: Arc<ER>,
    stats_repository: Arc<S>,
    email_port: Arc<E>,
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<C>,
    authentication_service: Arc<AS>,
    profile_repository: Arc<PF>,
    account_online_status_port: Arc<AC>,
    profile_picture_repo: Arc<PFP>,
    puzzle_repository: Arc<PZR>,
    chat_repository: Arc<CR>,
    rating_history_repository: Arc<RH>,
    tournament_repository: Arc<TR>,
    tournament_player_registration_repository: Arc<TPR>,
    match_repository: Arc<MR>,
    tournament_round_repository: Arc<TRR>,
) -> Application {
    let seek_service = Arc::new(SeekServiceImpl::new());
    let game_service = Arc::new(GameServiceImpl::new());
    let spectator_service = Arc::new(SpectatorServiceImpl::new());
    let chat_room_service = Arc::new(ChatRoomServiceImpl::new());
    let game_history_service = Arc::new(GameHistoryServiceImpl::new());
    let rating_service = Arc::new(RatingServiceImpl::new());
    let chat_content_policy = Arc::new(RustrictContentPolicy::new());
    let match_readiness_service = Arc::new(MatchReadinessServiceImpl::new());

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

    let tournament_match_workflow = Arc::new(TournamentMatchWorkflowImpl::new(
        tournament_player_registration_repository.clone(),
    ));

    let finalize_game_workflow = Arc::new(FinalizeGameWorkflowImpl::new(
        game_repository.clone(),
        rating_service.clone(),
        rating_repository.clone(),
        game_history_service.clone(),
        match_repository.clone(),
        notify_player_workflow.clone(),
        spectator_service.clone(),
        listener_notification_port.clone(),
        get_account_workflow.clone(),
        stats_repository.clone(),
        rating_history_repository.clone(),
        tournament_match_workflow.clone(),
    ));
    let observe_game_timeout_use_case = Arc::new(ObserveGameTimeoutUseCaseImpl::new(
        game_service.clone(),
        finalize_game_workflow.clone(),
    ));
    let game_timeout_scheduler = Arc::new(GameTimeoutRunnerImpl::new(
        observe_game_timeout_use_case.clone(),
    ));
    let player_disconnect_timeout_scheduler = Arc::new(DisconnectTimeoutRunnerImpl::new(
        observe_game_timeout_use_case.clone(),
    ));

    let create_game_from_match_workflow = Arc::new(CreateGameFromMatchWorkflowImpl::new(
        match_repository.clone(),
        game_history_service.clone(),
        game_repository.clone(),
        game_service.clone(),
        game_timeout_scheduler.clone(),
        listener_notification_port.clone(),
        get_snapshot_workflow.clone(),
    ));

    let shutdown_workflow = Arc::new(ShutdownWorkflowImpl::new(
        finalize_game_workflow.clone(),
        game_service.clone(),
    ));

    let application = Application {
        seek_accept_use_case: Arc::new(AcceptSeekUseCaseImpl::new(
            seek_service.clone(),
            match_repository.clone(),
            listener_notification_port.clone(),
            create_game_from_match_workflow.clone(),
        )),
        seek_cancel_use_case: Arc::new(CancelSeekUseCaseImpl::new(
            seek_service.clone(),
            listener_notification_port.clone(),
        )),
        seek_create_use_case: Arc::new(CreateSeekUseCaseImpl::new(
            seek_service.clone(),
            listener_notification_port.clone(),
        )),
        seek_get_use_case: Arc::new(GetSeekUseCaseImpl::new(seek_service.clone())),
        seek_list_use_case: Arc::new(ListSeeksUseCaseImpl::new(seek_service.clone())),

        match_readiness_use_case: Arc::new(MatchReadinessUseCaseImpl::new(
            match_repository.clone(),
            create_game_from_match_workflow.clone(),
            match_readiness_service.clone(),
            notify_player_workflow.clone(),
        )),

        account_set_online_use_case: Arc::new(SetAccountOnlineUseCaseImpl::new(
            account_online_status_port.clone(),
            listener_notification_port.clone(),
            seek_service.clone(),
            player_resolver_service.clone(),
            player_disconnect_timeout_scheduler.clone(),
            match_readiness_service.clone(),
            match_repository.clone(),
            notify_player_workflow.clone(),
        )),
        account_get_online_use_case: Arc::new(GetOnlineAccountsUseCaseImpl::new(
            account_online_status_port.clone(),
        )),

        listener_disconnect_use_case: Arc::new(ListenerDisconnectUseCaseImpl::new(
            spectator_service.clone(),
            chat_room_service.clone(),
        )),

        player_get_rating_use_case: Arc::new(PlayerGetRatingUseCaseImpl::new(
            rating_repository.clone(),
            rating_history_repository.clone(),
            rating_service.clone(),
        )),
        player_resolver_service,

        game_do_action_use_case: Arc::new(DoActionUseCaseImpl::new(
            game_service.clone(),
            notify_player_workflow.clone(),
            finalize_game_workflow.clone(),
        )),
        game_get_ongoing_use_case: Arc::new(GetOngoingGameUseCaseImpl::new(game_service.clone())),
        game_list_ongoing_use_case: Arc::new(ListOngoingGameUseCaseImpl::new(game_service.clone())),
        game_observe_use_case: Arc::new(ObserveGameUseCaseImpl::new(
            game_service.clone(),
            spectator_service.clone(),
        )),

        game_history_query_use_case: Arc::new(GameHistoryQueryUseCaseImpl::new(
            game_repository.clone(),
        )),

        chat_message_use_case: Arc::new(ChatMessageUseCaseImpl::new(
            listener_notification_port.clone(),
            player_connection_port.clone(),
            chat_room_service.clone(),
            chat_content_policy.clone(),
            chat_repository.clone(),
        )),
        chat_room_use_case: Arc::new(ChatRoomUseCaseImpl::new(chat_room_service.clone())),

        account_moderate_use_case: Arc::new(ModeratePlayerUseCaseImpl::new(
            email_port.clone(),
            policies,
            authentication_service.clone(),
        )),

        event_list_use_case: Arc::new(ListEventsUseCaseImpl::new(event_repository.clone())),

        get_snapshot_workflow,
        get_account_workflow,
        get_profile_use_case: Arc::new(GetProfileUseCaseImpl::new(
            profile_repository.clone(),
            profile_picture_repo.clone(),
        )),
        update_profile_use_case: Arc::new(UpdateProfileUseCaseImpl::new(
            profile_repository.clone(),
            authentication_service.clone(),
            profile_picture_repo.clone(),
        )),

        get_stats_use_case: Arc::new(GetPlayerStatsUseCaseImpl::new(
            stats_repository.clone(),
            rating_repository.clone(),
        )),

        get_puzzle_use_case: Arc::new(GetPuzzleUseCaseImpl::new(puzzle_repository.clone())),
        solve_puzzle_use_case: Arc::new(SolvePuzzleUseCaseImpl::new(puzzle_repository.clone())),

        get_tournaments_use_case: Arc::new(GetTournamentUseCaseImpl::new(
            tournament_repository.clone(),
            tournament_player_registration_repository.clone(),
            tournament_round_repository.clone(),
        )),
        tournament_player_registration_use_case: Arc::new(
            TournamentPlayerRegistrationUseCaseImpl::new(
                tournament_repository.clone(),
                tournament_player_registration_repository.clone(),
            ),
        ),
        host_tournament_use_case: Arc::new(HostTournamentUseCaseImpl::new(
            tournament_repository.clone(),
            match_repository.clone(),
            tournament_player_registration_repository.clone(),
            tournament_round_repository.clone(),
            rating_repository.clone(),
        )),

        match_get_use_case: Arc::new(GetMatchUseCaseImpl::new(match_repository.clone())),

        shutdown_workflow,
    };

    application
}
