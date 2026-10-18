use std::sync::Arc;

use crate::{
    ports::{
        authentication::AccountPort,
        chat::ChatRepository,
        connection::{UserConnectionPort, UserOnlineStatusPort},
        email::EmailPort,
        event::EventRepository,
        game_history::GameRepository,
        matches::MatchRepository,
        moderation::ModerationRepository,
        notification::ListenerNotificationPort,
        profile::{AccountProfileRepository, ProfilePictureRepository},
        puzzle::PuzzleRepository,
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
        moderation::moderate::{
            ModeratePlayerUseCase, ModeratePlayerUseCaseImpl, ModerationPolicies,
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
use tak_domain::{
    chat::{ChatRoomServiceImpl, RustrictContentPolicy},
    game::GameServiceImpl,
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
    pub game_get_ongoing_use_case: Arc<dyn GetOngoingGameUseCase + Send + Sync + 'static>,
    pub game_list_ongoing_use_case: Arc<dyn ListOngoingGameUseCase + Send + Sync + 'static>,
    pub game_observe_use_case: Arc<dyn ObserveGameUseCase + Send + Sync + 'static>,

    pub game_history_query_use_case: Arc<dyn GameHistoryQueryUseCase + Send + Sync + 'static>,

    pub chat_message_use_case: Arc<dyn ChatMessageUseCase + Send + Sync + 'static>,
    pub chat_room_use_case: Arc<dyn ChatRoomUseCase + Send + Sync + 'static>,

    pub account_moderate_use_case: Arc<dyn ModeratePlayerUseCase + Send + Sync + 'static>,

    pub event_list_use_case: Arc<dyn ListEventsUseCase + Send + Sync + 'static>,

    pub get_snapshot_workflow: Arc<dyn GetSnapshotWorkflow + Send + Sync + 'static>,
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

    pub authentication_use_case: Arc<dyn AuthenticationUseCase + Send + Sync + 'static>,
}

pub struct PortsCollectionImpl<
    G: GameRepository,
    R: RatingRepository,
    E: EventRepository,
    S: StatsRepository,
    PF: AccountProfileRepository,
    PFP: ProfilePictureRepository,
    PZR: PuzzleRepository,
    CR: ChatRepository<TrCtx>,
    RH: RatingHistoryRepository,
    TO: TournamentRepository,
    TPR: TournamentPlayerRepository,
    MR: MatchRepository<TrCtx>,
    TRR: TournamentRoundRepository,
    MO: ModerationRepository,
    TR: TransactionManager<TrCtx>,
    TrCtx,
> {
    pub game_repository: Arc<G>,
    pub rating_repository: Arc<R>,
    pub event_repository: Arc<E>,
    pub stats_repository: Arc<S>,
    pub profile_repository: Arc<PF>,
    pub profile_picture_repository: Arc<PFP>,
    pub puzzle_repository: Arc<PZR>,
    pub chat_repository: Arc<CR>,
    pub rating_history_repository: Arc<RH>,
    pub tournament_repository: Arc<TO>,
    pub tournament_player_repository: Arc<TPR>,
    pub match_repository: Arc<MR>,
    pub tournament_round_repository: Arc<TRR>,
    pub moderation_repository: Arc<MO>,
    pub transaction_manager: Arc<TR>,
    ctx: std::marker::PhantomData<TrCtx>,
}

impl<
    G: GameRepository,
    R: RatingRepository,
    E: EventRepository,
    S: StatsRepository,
    PF: AccountProfileRepository,
    PFP: ProfilePictureRepository,
    PZR: PuzzleRepository,
    CR: ChatRepository<TrCtx>,
    RH: RatingHistoryRepository,
    TO: TournamentRepository,
    TPR: TournamentPlayerRepository,
    MR: MatchRepository<TrCtx>,
    TRR: TournamentRoundRepository,
    MO: ModerationRepository,
    TR: TransactionManager<TrCtx>,
    TrCtx,
> PortsCollectionImpl<G, R, E, S, PF, PFP, PZR, CR, RH, TO, TPR, MR, TRR, MO, TR, TrCtx>
{
    pub fn new(
        game_repository: Arc<G>,
        rating_repository: Arc<R>,
        event_repository: Arc<E>,
        stats_repository: Arc<S>,
        profile_repository: Arc<PF>,
        profile_picture_repository: Arc<PFP>,
        puzzle_repository: Arc<PZR>,
        chat_repository: Arc<CR>,
        rating_history_repository: Arc<RH>,
        tournament_repository: Arc<TO>,
        tournament_player_repository: Arc<TPR>,
        match_repository: Arc<MR>,
        tournament_round_repository: Arc<TRR>,
        moderation_repository: Arc<MO>,
        transaction_manager: Arc<TR>,
    ) -> Self {
        Self {
            game_repository,
            rating_repository,
            event_repository,
            stats_repository,
            profile_repository,
            profile_picture_repository,
            puzzle_repository,
            chat_repository,
            rating_history_repository,
            tournament_repository,
            tournament_player_repository,
            match_repository,
            tournament_round_repository,
            moderation_repository,
            transaction_manager,
            ctx: std::marker::PhantomData,
        }
    }
}

pub trait PortsCollection {
    type GameRepo: GameRepository + Send + Sync + 'static;
    type RatingRepo: RatingRepository + Send + Sync + 'static;
    type EventRepo: EventRepository + Send + Sync + 'static;
    type StatsRepo: StatsRepository + Send + Sync + 'static;
    type ProfileRepo: AccountProfileRepository + Send + Sync + 'static;
    type ProfilePictureRepo: ProfilePictureRepository + Send + Sync + 'static;
    type PuzzleRepo: PuzzleRepository + Send + Sync + 'static;
    type ChatRepo: ChatRepository<Self::TrCtx> + Send + Sync + 'static;
    type RatingHistoryRepo: RatingHistoryRepository + Send + Sync + 'static;
    type TournamentRepo: TournamentRepository + Send + Sync + 'static;
    type TournamentPlayerRepo: TournamentPlayerRepository + Send + Sync + 'static;
    type MatchRepo: MatchRepository<Self::TrCtx> + Send + Sync + 'static;
    type TournamentRoundRepo: TournamentRoundRepository + Send + Sync + 'static;
    type ModerationRepo: ModerationRepository + Send + Sync + 'static;
    type TransactionManagerType: TransactionManager<Self::TrCtx> + Send + Sync + 'static;
    type TrCtx: Send + Sync + 'static;
    fn game_repository(&self) -> Arc<Self::GameRepo>;
    fn rating_repository(&self) -> Arc<Self::RatingRepo>;
    fn event_repository(&self) -> Arc<Self::EventRepo>;
    fn stats_repository(&self) -> Arc<Self::StatsRepo>;
    fn profile_repository(&self) -> Arc<Self::ProfileRepo>;
    fn profile_picture_repository(&self) -> Arc<Self::ProfilePictureRepo>;
    fn puzzle_repository(&self) -> Arc<Self::PuzzleRepo>;
    fn chat_repository(&self) -> Arc<Self::ChatRepo>;
    fn rating_history_repository(&self) -> Arc<Self::RatingHistoryRepo>;
    fn tournament_repository(&self) -> Arc<Self::TournamentRepo>;
    fn tournament_player_repository(&self) -> Arc<Self::TournamentPlayerRepo>;
    fn match_repository(&self) -> Arc<Self::MatchRepo>;
    fn tournament_round_repository(&self) -> Arc<Self::TournamentRoundRepo>;
    fn moderation_repository(&self) -> Arc<Self::ModerationRepo>;
    fn transaction_manager(&self) -> Arc<Self::TransactionManagerType>;
}

impl<
    G: GameRepository + Send + Sync + 'static,
    R: RatingRepository + Send + Sync + 'static,
    E: EventRepository + Send + Sync + 'static,
    S: StatsRepository + Send + Sync + 'static,
    PF: AccountProfileRepository + Send + Sync + 'static,
    PFP: ProfilePictureRepository + Send + Sync + 'static,
    PZR: PuzzleRepository + Send + Sync + 'static,
    CR: ChatRepository<TrCtx> + Send + Sync + 'static,
    RH: RatingHistoryRepository + Send + Sync + 'static,
    TO: TournamentRepository + Send + Sync + 'static,
    TPR: TournamentPlayerRepository + Send + Sync + 'static,
    MR: MatchRepository<TrCtx> + Send + Sync + 'static,
    TRR: TournamentRoundRepository + Send + Sync + 'static,
    MO: ModerationRepository + Send + Sync + 'static,
    TR: TransactionManager<TrCtx> + Send + Sync + 'static,
    TrCtx: Send + Sync + 'static,
> PortsCollection
    for PortsCollectionImpl<G, R, E, S, PF, PFP, PZR, CR, RH, TO, TPR, MR, TRR, MO, TR, TrCtx>
{
    type GameRepo = G;
    type RatingRepo = R;
    type EventRepo = E;
    type StatsRepo = S;
    type ProfileRepo = PF;
    type ProfilePictureRepo = PFP;
    type PuzzleRepo = PZR;
    type ChatRepo = CR;
    type RatingHistoryRepo = RH;
    type TournamentRepo = TO;
    type TournamentPlayerRepo = TPR;
    type MatchRepo = MR;
    type TournamentRoundRepo = TRR;
    type ModerationRepo = MO;
    type TransactionManagerType = TR;
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
    fn puzzle_repository(&self) -> Arc<Self::PuzzleRepo> {
        self.puzzle_repository.clone()
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
    fn transaction_manager(&self) -> Arc<Self::TransactionManagerType> {
        self.transaction_manager.clone()
    }
}

pub async fn build_application<
    L: ListenerNotificationPort + Send + Sync + 'static,
    C: UserConnectionPort + Send + Sync + 'static,
    G: GameRepository + Send + Sync + 'static,
    R: RatingRepository + Send + Sync + 'static,
    S: StatsRepository + Send + Sync + 'static,
    AS: AccountPort + Send + Sync + 'static,
    E: EmailPort + Send + Sync + 'static,
    ER: EventRepository + Send + Sync + 'static,
    PF: AccountProfileRepository + Send + Sync + 'static,
    AC: UserOnlineStatusPort + Send + Sync + 'static,
    PFP: ProfilePictureRepository + Send + Sync + 'static,
    PZR: PuzzleRepository + Send + Sync + 'static,
    CR: ChatRepository<TrCtx> + Send + Sync + 'static,
    RH: RatingHistoryRepository + Send + Sync + 'static,
    TR: TournamentRepository + Send + Sync + 'static,
    TPR: TournamentPlayerRepository + Send + Sync + 'static,
    MR: MatchRepository<TrCtx> + Send + Sync + 'static,
    TRR: TournamentRoundRepository + Send + Sync + 'static,
    MO: ModerationRepository + Send + Sync + 'static,
    TRA: TransactionManager<TrCtx> + Send + Sync + 'static,
    Ports: PortsCollection + Send + Sync + 'static,
    TrCtx: Send + Sync + 'static,
>(
    game_repository: Arc<G>,
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
    tournament_player_repository: Arc<TPR>,
    match_repository: Arc<MR>,
    tournament_round_repository: Arc<TRR>,
    moderation_repository: Arc<MO>,
    transaction_manager: Arc<TRA>,
    ports: Arc<Ports>,
) -> Application {
    let seek_service = Arc::new(SeekServiceImpl::new());
    let game_service = Arc::new(GameServiceImpl::new());
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
        authentication_service.clone(),
        rating_repository.clone(),
        rating_service.clone(),
    ));

    let notify_player_workflow = Arc::new(NotifyPlayerWorkflowImpl::new(
        listener_notification_port.clone(),
        player_connection_port.clone(),
        game_service.clone(),
        spectator_service.clone(),
    ));

    let tournament_match_workflow = Arc::new(TournamentMatchWorkflowImpl::new(
        tournament_player_repository.clone(),
    ));

    let finalize_game_workflow = Arc::new(FinalizeGameWorkflowImpl::new(
        transaction_manager.clone(),
        game_repository.clone(),
        rating_service.clone(),
        rating_repository.clone(),
        match_repository.clone(),
        notify_player_workflow.clone(),
        spectator_service.clone(),
        listener_notification_port.clone(),
        moderation_repository.clone(),
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
        ports.clone(),
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

        player_get_rating_use_case: Arc::new(PlayerGetRatingUseCaseImpl::new(
            rating_repository.clone(),
            rating_history_repository.clone(),
        )),

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
            transaction_manager.clone(),
        )),
        chat_room_use_case: Arc::new(ChatRoomUseCaseImpl::new(chat_room_service.clone())),

        account_moderate_use_case: Arc::new(ModeratePlayerUseCaseImpl::new(
            email_port.clone(),
            policies,
            moderation_repository.clone(),
            authentication_service.clone(),
        )),

        event_list_use_case: Arc::new(ListEventsUseCaseImpl::new(event_repository.clone())),

        get_snapshot_workflow,
        get_profile_use_case: Arc::new(GetProfileUseCaseImpl::new(
            profile_repository.clone(),
            profile_picture_repo.clone(),
        )),
        update_profile_use_case: Arc::new(UpdateProfileUseCaseImpl::new(
            profile_repository.clone(),
            moderation_repository.clone(),
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
            tournament_player_repository.clone(),
            tournament_round_repository.clone(),
        )),
        tournament_player_registration_use_case: Arc::new(
            TournamentPlayerRegistrationUseCaseImpl::new(
                tournament_repository.clone(),
                tournament_player_repository.clone(),
            ),
        ),
        host_tournament_use_case: Arc::new(HostTournamentUseCaseImpl::new(ports.clone())),

        match_get_use_case: Arc::new(GetMatchUseCaseImpl::new(ports.clone())),

        shutdown_workflow,

        authentication_use_case: Arc::new(AuthenticationUseCaseImpl::new(
            authentication_service.clone(),
            email_port.clone(),
            moderation_repository.clone(),
        )),
    };

    application
}
