use std::sync::Arc;

use crate::{
    ports::{
        authentication::AccountPort,
        chat::ChatRepository,
        connection::{UserConnectionPort, UserOnlineStatusPort},
        email::EmailPort,
        event::EventRepository,
        game::{GameHistoryRepository, GameRepository, GameStateRepository},
        matches::MatchRepository,
        moderation::ModerationRepository,
        notification::ListenerNotificationPort,
        profile::{AccountProfileRepository, ProfilePictureRepository},
        rating::RatingRepository,
        stats::{RatingHistoryRepository, StatsRepository},
        tournament::{TournamentPlayerRepository, TournamentRepository, TournamentRoundRepository},
        transaction::TransactionManager,
    },
    processes::{
        disconnect_timeout_runner::DisconnectTimeoutRunnerImpl,
        game_timeout_runner::GameTimeoutRunnerImpl,
    },
    workflow::{
        account::{
            authentication::{AuthenticationUseCase, AuthenticationUseCaseImpl},
            get_online::{GetOnlineUsersUseCase, GetOnlineUsersUseCaseImpl},
            get_profile::{GetProfileUseCase, GetProfileUseCaseImpl},
            get_snapshot::{GetSnapshotWorkflow, GetSnapshotWorkflowImpl},
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
            lifecycle::LifecycleGameWorkflowImpl,
            observe::{ObserveGameUseCase, ObserveGameUseCaseImpl},
            query::{GameQueryUseCase, GameQueryUseCaseImpl},
        },
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
        moderation::moderate::{
            ModeratePlayerUseCase, ModeratePlayerUseCaseImpl, ModerationPolicies,
        },
        player::{
            get_rating::{PlayerGetRatingUseCase, PlayerGetRatingUseCaseImpl},
            get_stats::{GetPlayerStatsUseCase, GetPlayerStatsUseCaseImpl},
            notify_player::NotifyPlayerWorkflowImpl,
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
use tak_domain::{
    chat::{ChatRoomServiceImpl, RustrictContentPolicy},
    matches::MatchReadinessServiceImpl,
    moderation::{AdminUserPolicy, HigherRoleUserPolicy, ModeratorUserPolicy},
    rating::RatingServiceImpl,
    seek::SeekServiceImpl,
    spectator::SpectatorServiceImpl,
};

pub mod ports;
pub mod processes;
pub mod workflow;

pub struct Application {
    pub seek_accept_use_case: Arc<dyn AcceptSeekUseCase + Send + Sync + 'static>,
    pub seek_cancel_use_case: Arc<dyn CancelSeekUseCase + Send + Sync + 'static>,
    pub seek_create_use_case: Arc<dyn CreateSeekUseCase + Send + Sync + 'static>,
    pub seek_get_use_case: Arc<dyn GetSeekUseCase + Send + Sync + 'static>,
    pub seek_list_use_case: Arc<dyn ListSeeksUseCase + Send + Sync + 'static>,
    pub match_readiness_use_case: Arc<dyn MatchReadinessUseCase + Send + Sync + 'static>,

    pub account_set_online_use_case: Arc<dyn SetAccountOnlineUseCase + Send + Sync + 'static>,
    pub account_get_online_use_case: Arc<dyn GetOnlineUsersUseCase + Send + Sync + 'static>,

    pub listener_disconnect_use_case: Arc<dyn ListenerDisconnectUseCase + Send + Sync + 'static>,

    pub player_get_rating_use_case: Arc<dyn PlayerGetRatingUseCase + Send + Sync + 'static>,

    pub game_do_action_use_case: Arc<dyn DoActionUseCase + Send + Sync + 'static>,
    pub game_observe_use_case: Arc<dyn ObserveGameUseCase + Send + Sync + 'static>,

    pub game_query_use_case: Arc<dyn GameQueryUseCase + Send + Sync + 'static>,

    pub chat_message_use_case: Arc<dyn ChatMessageUseCase + Send + Sync + 'static>,
    pub chat_room_use_case: Arc<dyn ChatRoomUseCase + Send + Sync + 'static>,

    pub account_moderate_use_case: Arc<dyn ModeratePlayerUseCase + Send + Sync + 'static>,

    pub event_list_use_case: Arc<dyn ListEventsUseCase + Send + Sync + 'static>,

    pub get_snapshot_workflow: Arc<dyn GetSnapshotWorkflow + Send + Sync + 'static>,
    pub get_profile_use_case: Arc<dyn GetProfileUseCase + Send + Sync + 'static>,
    pub update_profile_use_case: Arc<dyn UpdateProfileUseCase + Send + Sync + 'static>,

    pub get_stats_use_case: Arc<dyn GetPlayerStatsUseCase + Send + Sync + 'static>,

    pub get_tournaments_use_case: Arc<dyn GetTournamentUseCase + Send + Sync + 'static>,
    pub tournament_player_registration_use_case:
        Arc<dyn TournamentPlayerRegistrationUseCase + Send + Sync + 'static>,
    pub host_tournament_use_case: Arc<dyn HostTournamentUseCase + Send + Sync + 'static>,

    pub match_get_use_case: Arc<dyn GetMatchUseCase + Send + Sync + 'static>,

    pub shutdown_workflow: Arc<dyn ShutdownWorkflow + Send + Sync + 'static>,

    pub authentication_use_case: Arc<dyn AuthenticationUseCase + Send + Sync + 'static>,
}

pub trait PortsCollection {
    type GameRepo: GameRepository<Self::TrCtx> + Send + Sync + 'static;
    type RatingRepo: RatingRepository<Self::TrCtx> + Send + Sync + 'static;
    type EventRepo: EventRepository + Send + Sync + 'static;
    type StatsRepo: StatsRepository<Self::TrCtx> + Send + Sync + 'static;
    type ProfileRepo: AccountProfileRepository<Self::TrCtx> + Send + Sync + 'static;
    type ProfilePictureRepo: ProfilePictureRepository + Send + Sync + 'static;
    type ChatRepo: ChatRepository<Self::TrCtx> + Send + Sync + 'static;
    type RatingHistoryRepo: RatingHistoryRepository<Self::TrCtx> + Send + Sync + 'static;
    type TournamentRepo: TournamentRepository<Self::TrCtx> + Send + Sync + 'static;
    type TournamentPlayerRepo: TournamentPlayerRepository<Self::TrCtx> + Send + Sync + 'static;
    type MatchRepo: MatchRepository<Self::TrCtx> + Send + Sync + 'static;
    type TournamentRoundRepo: TournamentRoundRepository<Self::TrCtx> + Send + Sync + 'static;
    type ModerationRepo: ModerationRepository<Self::TrCtx> + Send + Sync + 'static;
    type TransactionMgr: TransactionManager<Self::TrCtx> + Send + Sync + 'static;
    type GameStateRepo: GameStateRepository<Self::TrCtx> + Send + Sync + 'static;
    type GameHistoryRepo: GameHistoryRepository<Self::TrCtx> + Send + Sync + 'static;
    type TrCtx: Send + Sync + 'static;
    fn game_repository(&self) -> Arc<Self::GameRepo>;
    fn rating_repository(&self) -> Arc<Self::RatingRepo>;
    fn event_repository(&self) -> Arc<Self::EventRepo>;
    fn stats_repository(&self) -> Arc<Self::StatsRepo>;
    fn profile_repository(&self) -> Arc<Self::ProfileRepo>;
    fn profile_picture_repository(&self) -> Arc<Self::ProfilePictureRepo>;
    fn chat_repository(&self) -> Arc<Self::ChatRepo>;
    fn rating_history_repository(&self) -> Arc<Self::RatingHistoryRepo>;
    fn tournament_repository(&self) -> Arc<Self::TournamentRepo>;
    fn tournament_player_repository(&self) -> Arc<Self::TournamentPlayerRepo>;
    fn match_repository(&self) -> Arc<Self::MatchRepo>;
    fn tournament_round_repository(&self) -> Arc<Self::TournamentRoundRepo>;
    fn moderation_repository(&self) -> Arc<Self::ModerationRepo>;
    fn transaction_manager(&self) -> Arc<Self::TransactionMgr>;
    fn game_state_repository(&self) -> Arc<Self::GameStateRepo>;
    fn game_history_repository(&self) -> Arc<Self::GameHistoryRepo>;
}

pub struct PortsCollectionImpl<
    GameRepo: GameRepository<TrCtx>,
    RatingRepo: RatingRepository<TrCtx>,
    EventRepo: EventRepository,
    StatsRepo: StatsRepository<TrCtx>,
    AccountProfileRepo: AccountProfileRepository<TrCtx>,
    ProfilePictureRepo: ProfilePictureRepository,
    ChatRepo: ChatRepository<TrCtx>,
    RatingHistoryRepo: RatingHistoryRepository<TrCtx>,
    TournamentRepo: TournamentRepository<TrCtx>,
    TournamentPlayerRepo: TournamentPlayerRepository<TrCtx>,
    MatchRepo: MatchRepository<TrCtx>,
    TournamentRoundRepo: TournamentRoundRepository<TrCtx>,
    ModerationRepo: ModerationRepository<TrCtx>,
    GameStateRepo: GameStateRepository<TrCtx>,
    GameHistoryRepo: GameHistoryRepository<TrCtx>,
    TransactionMgr: TransactionManager<TrCtx>,
    TrCtx,
> {
    pub game_repository: Arc<GameRepo>,
    pub rating_repository: Arc<RatingRepo>,
    pub event_repository: Arc<EventRepo>,
    pub stats_repository: Arc<StatsRepo>,
    pub profile_repository: Arc<AccountProfileRepo>,
    pub profile_picture_repository: Arc<ProfilePictureRepo>,
    pub chat_repository: Arc<ChatRepo>,
    pub rating_history_repository: Arc<RatingHistoryRepo>,
    pub tournament_repository: Arc<TournamentRepo>,
    pub tournament_player_repository: Arc<TournamentPlayerRepo>,
    pub match_repository: Arc<MatchRepo>,
    pub tournament_round_repository: Arc<TournamentRoundRepo>,
    pub moderation_repository: Arc<ModerationRepo>,
    pub game_state_repository: Arc<GameStateRepo>,
    pub game_history_repository: Arc<GameHistoryRepo>,
    pub transaction_manager: Arc<TransactionMgr>,
    ctx: std::marker::PhantomData<TrCtx>,
}

impl<
    GameRepo: GameRepository<TrCtx>,
    RatingRepo: RatingRepository<TrCtx>,
    EventRepo: EventRepository,
    StatsRepo: StatsRepository<TrCtx>,
    AccountProfileRepo: AccountProfileRepository<TrCtx>,
    ProfilePictureRepo: ProfilePictureRepository,
    ChatRepo: ChatRepository<TrCtx>,
    RatingHistoryRepo: RatingHistoryRepository<TrCtx>,
    TournamentRepo: TournamentRepository<TrCtx>,
    TournamentPlayerRepo: TournamentPlayerRepository<TrCtx>,
    MatchRepo: MatchRepository<TrCtx>,
    TournamentRoundRepo: TournamentRoundRepository<TrCtx>,
    ModerationRepo: ModerationRepository<TrCtx>,
    GameStateRepo: GameStateRepository<TrCtx>,
    GameHistoryRepo: GameHistoryRepository<TrCtx>,
    TransactionMgr: TransactionManager<TrCtx>,
    TrCtx,
>
    PortsCollectionImpl<
        GameRepo,
        RatingRepo,
        EventRepo,
        StatsRepo,
        AccountProfileRepo,
        ProfilePictureRepo,
        ChatRepo,
        RatingHistoryRepo,
        TournamentRepo,
        TournamentPlayerRepo,
        MatchRepo,
        TournamentRoundRepo,
        ModerationRepo,
        GameStateRepo,
        GameHistoryRepo,
        TransactionMgr,
        TrCtx,
    >
{
    pub fn new(
        game_repository: Arc<GameRepo>,
        rating_repository: Arc<RatingRepo>,
        event_repository: Arc<EventRepo>,
        stats_repository: Arc<StatsRepo>,
        profile_repository: Arc<AccountProfileRepo>,
        profile_picture_repository: Arc<ProfilePictureRepo>,
        chat_repository: Arc<ChatRepo>,
        rating_history_repository: Arc<RatingHistoryRepo>,
        tournament_repository: Arc<TournamentRepo>,
        tournament_player_repository: Arc<TournamentPlayerRepo>,
        match_repository: Arc<MatchRepo>,
        tournament_round_repository: Arc<TournamentRoundRepo>,
        moderation_repository: Arc<ModerationRepo>,
        game_state_repository: Arc<GameStateRepo>,
        game_history_repository: Arc<GameHistoryRepo>,
        transaction_manager: Arc<TransactionMgr>,
    ) -> Self {
        Self {
            game_repository,
            rating_repository,
            event_repository,
            stats_repository,
            profile_repository,
            profile_picture_repository,
            chat_repository,
            rating_history_repository,
            tournament_repository,
            tournament_player_repository,
            match_repository,
            tournament_round_repository,
            moderation_repository,
            game_state_repository,
            game_history_repository,
            transaction_manager,
            ctx: std::marker::PhantomData,
        }
    }
}

impl<
    GameRepo: GameRepository<TrCtx> + Send + Sync + 'static,
    RatingRepo: RatingRepository<TrCtx> + Send + Sync + 'static,
    EventRepo: EventRepository + Send + Sync + 'static,
    StatsRepo: StatsRepository<TrCtx> + Send + Sync + 'static,
    ProfileRepo: AccountProfileRepository<TrCtx> + Send + Sync + 'static,
    ProfilePictureRepo: ProfilePictureRepository + Send + Sync + 'static,
    ChatRepo: ChatRepository<TrCtx> + Send + Sync + 'static,
    RatingHistoryRepo: RatingHistoryRepository<TrCtx> + Send + Sync + 'static,
    TournamentRepo: TournamentRepository<TrCtx> + Send + Sync + 'static,
    TournamentPlayerRepo: TournamentPlayerRepository<TrCtx> + Send + Sync + 'static,
    MatchRepo: MatchRepository<TrCtx> + Send + Sync + 'static,
    TournamentRoundRepo: TournamentRoundRepository<TrCtx> + Send + Sync + 'static,
    ModerationRepo: ModerationRepository<TrCtx> + Send + Sync + 'static,
    GameStateRepo: GameStateRepository<TrCtx> + Send + Sync + 'static,
    GameHistoryRepo: GameHistoryRepository<TrCtx> + Send + Sync + 'static,
    TransactionMgr: TransactionManager<TrCtx> + Send + Sync + 'static,
    TrCtx: Send + Sync + 'static,
> PortsCollection
    for PortsCollectionImpl<
        GameRepo,
        RatingRepo,
        EventRepo,
        StatsRepo,
        ProfileRepo,
        ProfilePictureRepo,
        ChatRepo,
        RatingHistoryRepo,
        TournamentRepo,
        TournamentPlayerRepo,
        MatchRepo,
        TournamentRoundRepo,
        ModerationRepo,
        GameStateRepo,
        GameHistoryRepo,
        TransactionMgr,
        TrCtx,
    >
{
    type GameRepo = GameRepo;
    type RatingRepo = RatingRepo;
    type EventRepo = EventRepo;
    type StatsRepo = StatsRepo;
    type ProfileRepo = ProfileRepo;
    type ProfilePictureRepo = ProfilePictureRepo;
    type ChatRepo = ChatRepo;
    type RatingHistoryRepo = RatingHistoryRepo;
    type TournamentRepo = TournamentRepo;
    type TournamentPlayerRepo = TournamentPlayerRepo;
    type MatchRepo = MatchRepo;
    type TournamentRoundRepo = TournamentRoundRepo;
    type ModerationRepo = ModerationRepo;
    type GameStateRepo = GameStateRepo;
    type GameHistoryRepo = GameHistoryRepo;
    type TransactionMgr = TransactionMgr;
    type TrCtx = TrCtx;

    fn game_repository(&self) -> Arc<Self::GameRepo> {
        self.game_repository.clone()
    }
    fn rating_repository(&self) -> Arc<Self::RatingRepo> {
        self.rating_repository.clone()
    }
    fn event_repository(&self) -> Arc<Self::EventRepo> {
        self.event_repository.clone()
    }
    fn stats_repository(&self) -> Arc<Self::StatsRepo> {
        self.stats_repository.clone()
    }
    fn profile_repository(&self) -> Arc<Self::ProfileRepo> {
        self.profile_repository.clone()
    }
    fn profile_picture_repository(&self) -> Arc<Self::ProfilePictureRepo> {
        self.profile_picture_repository.clone()
    }
    fn chat_repository(&self) -> Arc<Self::ChatRepo> {
        self.chat_repository.clone()
    }
    fn rating_history_repository(&self) -> Arc<Self::RatingHistoryRepo> {
        self.rating_history_repository.clone()
    }
    fn tournament_repository(&self) -> Arc<Self::TournamentRepo> {
        self.tournament_repository.clone()
    }
    fn tournament_player_repository(&self) -> Arc<Self::TournamentPlayerRepo> {
        self.tournament_player_repository.clone()
    }
    fn match_repository(&self) -> Arc<Self::MatchRepo> {
        self.match_repository.clone()
    }
    fn tournament_round_repository(&self) -> Arc<Self::TournamentRoundRepo> {
        self.tournament_round_repository.clone()
    }
    fn moderation_repository(&self) -> Arc<Self::ModerationRepo> {
        self.moderation_repository.clone()
    }
    fn game_state_repository(&self) -> Arc<Self::GameStateRepo> {
        self.game_state_repository.clone()
    }
    fn game_history_repository(&self) -> Arc<Self::GameHistoryRepo> {
        self.game_history_repository.clone()
    }
    fn transaction_manager(&self) -> Arc<Self::TransactionMgr> {
        self.transaction_manager.clone()
    }
}

pub async fn build_application<
    L: ListenerNotificationPort + Send + Sync + 'static,
    C: UserConnectionPort + Send + Sync + 'static,
    AS: AccountPort + Send + Sync + 'static,
    E: EmailPort + Send + Sync + 'static,
    ER: EventRepository + Send + Sync + 'static,
    AC: UserOnlineStatusPort + Send + Sync + 'static,
    Ports: PortsCollection + Send + Sync + 'static,
>(
    event_repository: Arc<ER>,
    email_port: Arc<E>,
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<C>,
    authentication_service: Arc<AS>,
    account_online_status_port: Arc<AC>,
    ports: Arc<Ports>,
) -> Application {
    let seek_service = Arc::new(SeekServiceImpl::new());
    let spectator_service = Arc::new(SpectatorServiceImpl::new());
    let chat_room_service = Arc::new(ChatRoomServiceImpl::new());
    let rating_service = Arc::new(RatingServiceImpl::new());
    let chat_content_policy = Arc::new(RustrictContentPolicy::new());
    let match_readiness_service = Arc::new(MatchReadinessServiceImpl::new());

    let policies = ModerationPolicies {
        ban_policy: Arc::new(AdminUserPolicy),
        silence_policy: Arc::new(ModeratorUserPolicy),
        set_moderator_policy: Arc::new(HigherRoleUserPolicy),
        set_admin_policy: Arc::new(HigherRoleUserPolicy),
        set_user_policy: Arc::new(HigherRoleUserPolicy),
    };

    let get_snapshot_workflow = Arc::new(GetSnapshotWorkflowImpl::new(
        ports.clone(),
        authentication_service.clone(),
        rating_service.clone(),
    ));

    let notify_player_workflow = Arc::new(NotifyPlayerWorkflowImpl::new(
        listener_notification_port.clone(),
        player_connection_port.clone(),
        spectator_service.clone(),
    ));

    let tournament_match_workflow = Arc::new(TournamentMatchWorkflowImpl::new(ports.clone()));

    let finalize_game_workflow = Arc::new(LifecycleGameWorkflowImpl::new(
        ports.clone(),
        rating_service.clone(),
        notify_player_workflow.clone(),
        spectator_service.clone(),
        listener_notification_port.clone(),
        tournament_match_workflow.clone(),
    ));
    let game_do_action_use_case = Arc::new(DoActionUseCaseImpl::new(
        ports.clone(),
        notify_player_workflow.clone(),
        finalize_game_workflow.clone(),
    ));
    let game_timeout_scheduler =
        Arc::new(GameTimeoutRunnerImpl::new(game_do_action_use_case.clone()));
    let player_disconnect_timeout_scheduler = Arc::new(DisconnectTimeoutRunnerImpl::new(
        game_do_action_use_case.clone(),
    ));

    let create_game_from_match_workflow = Arc::new(CreateGameFromMatchWorkflowImpl::new(
        ports.clone(),
        game_timeout_scheduler.clone(),
        listener_notification_port.clone(),
        get_snapshot_workflow.clone(),
    ));

    let shutdown_workflow = Arc::new(ShutdownWorkflowImpl::new());

    let application = Application {
        seek_accept_use_case: Arc::new(AcceptSeekUseCaseImpl::new(
            ports.clone(),
            seek_service.clone(),
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
            ports.clone(),
            create_game_from_match_workflow.clone(),
            match_readiness_service.clone(),
            notify_player_workflow.clone(),
        )),

        account_set_online_use_case: Arc::new(SetAccountOnlineUseCaseImpl::new(
            ports.clone(),
            account_online_status_port.clone(),
            listener_notification_port.clone(),
            seek_service.clone(),
            player_disconnect_timeout_scheduler.clone(),
            match_readiness_service.clone(),
            notify_player_workflow.clone(),
        )),
        account_get_online_use_case: Arc::new(GetOnlineUsersUseCaseImpl::new(
            account_online_status_port.clone(),
        )),

        listener_disconnect_use_case: Arc::new(ListenerDisconnectUseCaseImpl::new(
            spectator_service.clone(),
            chat_room_service.clone(),
        )),

        player_get_rating_use_case: Arc::new(PlayerGetRatingUseCaseImpl::new(ports.clone())),

        game_do_action_use_case,
        game_observe_use_case: Arc::new(ObserveGameUseCaseImpl::new(
            ports.clone(),
            spectator_service.clone(),
        )),

        game_query_use_case: Arc::new(GameQueryUseCaseImpl::new(ports.clone())),

        chat_message_use_case: Arc::new(ChatMessageUseCaseImpl::new(
            ports.clone(),
            listener_notification_port.clone(),
            player_connection_port.clone(),
            chat_room_service.clone(),
            chat_content_policy.clone(),
        )),
        chat_room_use_case: Arc::new(ChatRoomUseCaseImpl::new(chat_room_service.clone())),

        account_moderate_use_case: Arc::new(ModeratePlayerUseCaseImpl::new(
            ports.clone(),
            email_port.clone(),
            policies,
            authentication_service.clone(),
        )),

        event_list_use_case: Arc::new(ListEventsUseCaseImpl::new(event_repository.clone())),

        get_snapshot_workflow,
        get_profile_use_case: Arc::new(GetProfileUseCaseImpl::new(ports.clone())),
        update_profile_use_case: Arc::new(UpdateProfileUseCaseImpl::new(ports.clone())),

        get_stats_use_case: Arc::new(GetPlayerStatsUseCaseImpl::new(ports.clone())),

        get_tournaments_use_case: Arc::new(GetTournamentUseCaseImpl::new(ports.clone())),
        tournament_player_registration_use_case: Arc::new(
            TournamentPlayerRegistrationUseCaseImpl::new(ports.clone()),
        ),
        host_tournament_use_case: Arc::new(HostTournamentUseCaseImpl::new(ports.clone())),

        match_get_use_case: Arc::new(GetMatchUseCaseImpl::new(ports.clone())),

        shutdown_workflow,

        authentication_use_case: Arc::new(AuthenticationUseCaseImpl::new(
            ports.clone(),
            authentication_service.clone(),
            email_port.clone(),
        )),
    };

    application
}
