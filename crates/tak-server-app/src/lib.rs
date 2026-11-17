use std::sync::Arc;

use crate::{
    application::{
        authentication::{AuthenticationUseCase, AuthenticationUseCaseImpl},
        chat::{ChatUseCase, ChatUseCaseImpl},
        events::{EventsUseCase, EventsUseCaseImpl},
        gameplay::{GameplayUseCase, GameplayUseCaseImpl},
        listener::{ListenerUseCase, ListenerUseCaseImpl},
        matchmaking::{MatchmakingUseCase, MatchmakingUseCaseImpl},
        moderation::{ModerationUseCase, ModerationUseCaseImpl},
        player::{PlayerUseCase, PlayerUseCaseImpl},
        profile::{ProfileUseCase, ProfileUseCaseImpl},
        shutdown::{ShutdownUseCase, ShutdownUseCaseImpl},
        tournament::{TournamentUseCase, TournamentUseCaseImpl},
    },
    ports::{
        authentication::AccountPort,
        chat::{ChatRepository, ChatRoomServiceImpl},
        connection::{UserConnectionPort, UserOnlineStatusPort},
        email::EmailPort,
        event::EventRepository,
        game::{GameHistoryRepository, GameRepository, GameStateRepository},
        matches::{MatchReadinessServiceImpl, MatchRepository},
        moderation::ModerationRepository,
        notification::ListenerNotificationPort,
        profile::{AccountProfileRepository, ProfilePictureRepository},
        rating::RatingRepository,
        seeks::SeekServiceImpl,
        spectator::SpectatorServiceImpl,
        stats::{RatingHistoryRepository, StatsRepository},
        tournament::{TournamentPlayerRepository, TournamentRepository, TournamentRoundRepository},
        transaction::TransactionManager,
    },
    processes::{
        disconnect_timeout_runner::DisconnectTimeoutRunnerImpl,
        game_timeout_runner::GameTimeoutRunnerImpl,
    },
    workflow::{
        get_snapshot::GetSnapshotWorkflowImpl, lifecycle::LifecycleGameWorkflowImpl,
        notify_player::NotifyPlayerWorkflowImpl,
    },
};

pub mod application;
pub mod ports;
pub mod processes;
pub mod workflow;
pub mod errors;

pub struct Application {
    pub matchmaking_use_case: Arc<dyn MatchmakingUseCase + Send + Sync + 'static>,
    pub listener_use_case: Arc<dyn ListenerUseCase + Send + Sync + 'static>,
    pub player_use_case: Arc<dyn PlayerUseCase + Send + Sync + 'static>,
    pub gameplay_use_case: Arc<dyn GameplayUseCase + Send + Sync + 'static>,
    pub chat_use_case: Arc<dyn ChatUseCase + Send + Sync + 'static>,
    pub moderation_use_case: Arc<dyn ModerationUseCase + Send + Sync + 'static>,
    pub events_use_case: Arc<dyn EventsUseCase + Send + Sync + 'static>,
    pub profile_use_case: Arc<dyn ProfileUseCase + Send + Sync + 'static>,
    pub tournament_use_case: Arc<dyn TournamentUseCase + Send + Sync + 'static>,
    pub shutdown_use_case: Arc<dyn ShutdownUseCase + Send + Sync + 'static>,
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
    UO: UserOnlineStatusPort + Send + Sync + 'static,
    Ports: PortsCollection + Send + Sync + 'static,
>(
    email_port: Arc<E>,
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<C>,
    authentication_service: Arc<AS>,
    user_online_status_port: Arc<UO>,
    ports: Arc<Ports>,
) -> Application {
    let seek_service = Arc::new(SeekServiceImpl::new());
    let spectator_service = Arc::new(SpectatorServiceImpl::new());
    let chat_room_service = Arc::new(ChatRoomServiceImpl::new());
    let match_readiness_service = Arc::new(MatchReadinessServiceImpl::new());

    let get_snapshot_workflow = Arc::new(GetSnapshotWorkflowImpl::new(
        ports.clone(),
        authentication_service.clone(),
    ));

    let notify_player_workflow = Arc::new(NotifyPlayerWorkflowImpl::new(
        listener_notification_port.clone(),
        player_connection_port.clone(),
        spectator_service.clone(),
    ));

    let finalize_game_workflow = Arc::new(LifecycleGameWorkflowImpl::new(
        ports.clone(),
        notify_player_workflow.clone(),
        spectator_service.clone(),
        listener_notification_port.clone(),
    ));
    let game_timeout_scheduler =
        Arc::new(GameTimeoutRunnerImpl::new(finalize_game_workflow.clone()));
    let player_disconnect_timeout_scheduler = Arc::new(DisconnectTimeoutRunnerImpl::new(
        ports.clone(),
        finalize_game_workflow.clone(),
    ));

    let application = Application {
        matchmaking_use_case: Arc::new(MatchmakingUseCaseImpl::new(
            ports.clone(),
            seek_service.clone(),
            listener_notification_port.clone(),
            match_readiness_service.clone(),
            notify_player_workflow.clone(),
            game_timeout_scheduler.clone(),
            get_snapshot_workflow.clone(),
        )),
        listener_use_case: Arc::new(ListenerUseCaseImpl::new(
            spectator_service.clone(),
            chat_room_service.clone(),
        )),
        player_use_case: Arc::new(PlayerUseCaseImpl::new(
            ports.clone(),
            user_online_status_port.clone(),
            listener_notification_port.clone(),
            seek_service.clone(),
            player_disconnect_timeout_scheduler.clone(),
            match_readiness_service.clone(),
            notify_player_workflow.clone(),
            get_snapshot_workflow.clone(),
        )),
        gameplay_use_case: Arc::new(GameplayUseCaseImpl::new(
            ports.clone(),
            notify_player_workflow.clone(),
            finalize_game_workflow.clone(),
            spectator_service.clone(),
        )),
        chat_use_case: Arc::new(ChatUseCaseImpl::new(
            ports.clone(),
            listener_notification_port.clone(),
            player_connection_port.clone(),
            chat_room_service.clone(),
        )),
        moderation_use_case: Arc::new(ModerationUseCaseImpl::new(
            ports.clone(),
            email_port.clone(),
            authentication_service.clone(),
        )),
        events_use_case: Arc::new(EventsUseCaseImpl::new(ports.clone())),
        profile_use_case: Arc::new(ProfileUseCaseImpl::new(ports.clone())),
        tournament_use_case: Arc::new(TournamentUseCaseImpl::new(ports.clone())),
        authentication_use_case: Arc::new(AuthenticationUseCaseImpl::new(
            ports.clone(),
            authentication_service.clone(),
            email_port.clone(),
        )),
        shutdown_use_case: Arc::new(ShutdownUseCaseImpl::new()),
    };

    application
}
