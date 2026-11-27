use std::sync::Arc;

use crate::{
    application::{
        authentication::{AuthenticationUseCase, AuthenticationUseCaseImpl},
        chat::{ChatUseCase, ChatUseCaseImpl},
        events::{EventsUseCase, EventsUseCaseImpl},
        gameplay::{GameplayUseCase, GameplayUseCaseImpl},
        matchmaking::{MatchmakingUseCase, MatchmakingUseCaseImpl},
        moderation::{ModerationUseCase, ModerationUseCaseImpl},
        player::{PlayerUseCase, PlayerUseCaseImpl},
        profile::{ProfileUseCase, ProfileUseCaseImpl},
        shutdown::{ShutdownUseCase, ShutdownUseCaseImpl},
        tournament::{TournamentUseCase, TournamentUseCaseImpl},
    },
    ports::{
        authentication::AuthenticationPort,
        chat::ChatRepository,
        connection::{UserOnlineStatusPort, UserOnlineStatusService},
        email::EmailPort,
        event::EventRepository,
        game::{GameHistoryRepository, GameRepository, GameStateRepository},
        matches::{MatchReadinessService, MatchReadinessServiceImpl, MatchRepository},
        moderation::ModerationRepository,
        notification::NotificationPort,
        profile::{ProfilePictureRepository, ProfileRepository},
        rating::RatingRepository,
        seeks::{SeekService, SeekServiceImpl},
        stats::{RatingHistoryRepository, StatsRepository},
        tournament::{TournamentPlayerRepository, TournamentRepository, TournamentRoundRepository},
        transaction::TransactionManager,
    },
    processes::{
        disconnect_timeout_runner::DisconnectTimeoutRunnerImpl,
        game_timeout_runner::GameTimeoutRunnerImpl,
    },
};

pub mod application;
pub mod errors;
pub mod ports;
pub mod processes;
pub mod workflow;

pub struct Application {
    pub matchmaking_use_case: Arc<dyn MatchmakingUseCase + Send + Sync + 'static>,
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
    type RpCtx: Send + Sync + 'static;
    fn game_repository(&self) -> Arc<dyn GameRepository<Self::RpCtx> + Send + Sync + 'static>;
    fn rating_repository(&self) -> Arc<dyn RatingRepository<Self::RpCtx> + Send + Sync + 'static>;
    fn event_repository(&self) -> Arc<dyn EventRepository + Send + Sync + 'static>;
    fn stats_repository(&self) -> Arc<dyn StatsRepository<Self::RpCtx> + Send + Sync + 'static>;
    fn profile_repository(&self)
    -> Arc<dyn ProfileRepository<Self::RpCtx> + Send + Sync + 'static>;
    fn profile_picture_repository(
        &self,
    ) -> Arc<dyn ProfilePictureRepository + Send + Sync + 'static>;
    fn chat_repository(&self) -> Arc<dyn ChatRepository<Self::RpCtx> + Send + Sync + 'static>;
    fn rating_history_repository(
        &self,
    ) -> Arc<dyn RatingHistoryRepository<Self::RpCtx> + Send + Sync + 'static>;
    fn tournament_repository(
        &self,
    ) -> Arc<dyn TournamentRepository<Self::RpCtx> + Send + Sync + 'static>;
    fn tournament_player_repository(
        &self,
    ) -> Arc<dyn TournamentPlayerRepository<Self::RpCtx> + Send + Sync + 'static>;
    fn match_repository(&self) -> Arc<dyn MatchRepository<Self::RpCtx> + Send + Sync + 'static>;
    fn tournament_round_repository(
        &self,
    ) -> Arc<dyn TournamentRoundRepository<Self::RpCtx> + Send + Sync + 'static>;
    fn moderation_repository(
        &self,
    ) -> Arc<dyn ModerationRepository<Self::RpCtx> + Send + Sync + 'static>;
    fn game_state_repository(
        &self,
    ) -> Arc<dyn GameStateRepository<Self::RpCtx> + Send + Sync + 'static>;
    fn game_history_repository(
        &self,
    ) -> Arc<dyn GameHistoryRepository<Self::RpCtx> + Send + Sync + 'static>;
    fn transaction_manager(
        &self,
    ) -> Arc<dyn TransactionManager<Self::RpCtx> + Send + Sync + 'static>;

    fn email_port(&self) -> Arc<dyn EmailPort + Send + Sync + 'static>;
    fn notification_port(&self) -> Arc<dyn NotificationPort + Send + Sync + 'static>;
    fn authentication_port(&self) -> Arc<dyn AuthenticationPort + Send + Sync + 'static>;
    fn user_online_status_port(&self) -> Arc<dyn UserOnlineStatusPort + Send + Sync + 'static>;

    fn seek_service(&self) -> Arc<dyn SeekService + Send + Sync + 'static>;
    fn match_readiness_service(&self) -> Arc<dyn MatchReadinessService + Send + Sync + 'static>;
}

pub struct PortsCollectionImpl<TrCtx> {
    pub game_repository: Arc<dyn GameRepository<TrCtx> + Send + Sync + 'static>,
    pub rating_repository: Arc<dyn RatingRepository<TrCtx> + Send + Sync + 'static>,
    pub event_repository: Arc<dyn EventRepository + Send + Sync + 'static>,
    pub stats_repository: Arc<dyn StatsRepository<TrCtx> + Send + Sync + 'static>,
    pub profile_repository: Arc<dyn ProfileRepository<TrCtx> + Send + Sync + 'static>,
    pub profile_picture_repository: Arc<dyn ProfilePictureRepository + Send + Sync + 'static>,
    pub chat_repository: Arc<dyn ChatRepository<TrCtx> + Send + Sync + 'static>,
    pub rating_history_repository: Arc<dyn RatingHistoryRepository<TrCtx> + Send + Sync + 'static>,
    pub tournament_repository: Arc<dyn TournamentRepository<TrCtx> + Send + Sync + 'static>,
    pub tournament_player_repository:
        Arc<dyn TournamentPlayerRepository<TrCtx> + Send + Sync + 'static>,
    pub match_repository: Arc<dyn MatchRepository<TrCtx> + Send + Sync + 'static>,
    pub tournament_round_repository:
        Arc<dyn TournamentRoundRepository<TrCtx> + Send + Sync + 'static>,
    pub moderation_repository: Arc<dyn ModerationRepository<TrCtx> + Send + Sync + 'static>,
    pub game_state_repository: Arc<dyn GameStateRepository<TrCtx> + Send + Sync + 'static>,
    pub game_history_repository: Arc<dyn GameHistoryRepository<TrCtx> + Send + Sync + 'static>,
    pub transaction_manager: Arc<dyn TransactionManager<TrCtx> + Send + Sync + 'static>,

    pub email_port: Arc<dyn EmailPort + Send + Sync + 'static>,
    pub notification_port: Arc<dyn NotificationPort + Send + Sync + 'static>,
    pub authentication_port: Arc<dyn AuthenticationPort + Send + Sync + 'static>,
    pub user_online_status_port: Arc<dyn UserOnlineStatusPort + Send + Sync + 'static>,

    pub seek_service: Arc<dyn SeekService + Send + Sync + 'static>,
    pub match_readiness_service: Arc<dyn MatchReadinessService + Send + Sync + 'static>,
}

impl<TrCtx> PortsCollectionImpl<TrCtx> {
    pub fn new(
        game_repository: Arc<dyn GameRepository<TrCtx> + Send + Sync + 'static>,
        rating_repository: Arc<dyn RatingRepository<TrCtx> + Send + Sync + 'static>,
        event_repository: Arc<dyn EventRepository + Send + Sync + 'static>,
        stats_repository: Arc<dyn StatsRepository<TrCtx> + Send + Sync + 'static>,
        profile_repository: Arc<dyn ProfileRepository<TrCtx> + Send + Sync + 'static>,
        profile_picture_repository: Arc<dyn ProfilePictureRepository + Send + Sync + 'static>,
        chat_repository: Arc<dyn ChatRepository<TrCtx> + Send + Sync + 'static>,
        rating_history_repository: Arc<dyn RatingHistoryRepository<TrCtx> + Send + Sync + 'static>,
        tournament_repository: Arc<dyn TournamentRepository<TrCtx> + Send + Sync + 'static>,
        tournament_player_repository: Arc<
            dyn TournamentPlayerRepository<TrCtx> + Send + Sync + 'static,
        >,
        match_repository: Arc<dyn MatchRepository<TrCtx> + Send + Sync + 'static>,
        tournament_round_repository: Arc<
            dyn TournamentRoundRepository<TrCtx> + Send + Sync + 'static,
        >,
        moderation_repository: Arc<dyn ModerationRepository<TrCtx> + Send + Sync + 'static>,
        game_state_repository: Arc<dyn GameStateRepository<TrCtx> + Send + Sync + 'static>,
        game_history_repository: Arc<dyn GameHistoryRepository<TrCtx> + Send + Sync + 'static>,
        transaction_manager: Arc<dyn TransactionManager<TrCtx> + Send + Sync + 'static>,

        email_port: Arc<dyn EmailPort + Send + Sync + 'static>,
        notification_port: Arc<dyn NotificationPort + Send + Sync + 'static>,
        authentication_port: Arc<dyn AuthenticationPort + Send + Sync + 'static>,
        user_online_status_port: Arc<dyn UserOnlineStatusPort + Send + Sync + 'static>,

        seek_service: Arc<dyn SeekService + Send + Sync + 'static>,
        match_readiness_service: Arc<dyn MatchReadinessService + Send + Sync + 'static>,
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
            email_port,
            notification_port,
            authentication_port,
            user_online_status_port,
            seek_service,
            match_readiness_service,
        }
    }
}

impl<RpCtx: Send + Sync + 'static> PortsCollection for PortsCollectionImpl<RpCtx> {
    type RpCtx = RpCtx;
    fn game_repository(&self) -> Arc<dyn GameRepository<RpCtx> + Send + Sync + 'static> {
        self.game_repository.clone()
    }
    fn rating_repository(&self) -> Arc<dyn RatingRepository<RpCtx> + Send + Sync + 'static> {
        self.rating_repository.clone()
    }
    fn event_repository(&self) -> Arc<dyn EventRepository + Send + Sync + 'static> {
        self.event_repository.clone()
    }
    fn stats_repository(&self) -> Arc<dyn StatsRepository<RpCtx> + Send + Sync + 'static> {
        self.stats_repository.clone()
    }
    fn profile_repository(&self) -> Arc<dyn ProfileRepository<RpCtx> + Send + Sync + 'static> {
        self.profile_repository.clone()
    }
    fn profile_picture_repository(
        &self,
    ) -> Arc<dyn ProfilePictureRepository + Send + Sync + 'static> {
        self.profile_picture_repository.clone()
    }
    fn chat_repository(&self) -> Arc<dyn ChatRepository<RpCtx> + Send + Sync + 'static> {
        self.chat_repository.clone()
    }
    fn rating_history_repository(
        &self,
    ) -> Arc<dyn RatingHistoryRepository<RpCtx> + Send + Sync + 'static> {
        self.rating_history_repository.clone()
    }
    fn tournament_repository(
        &self,
    ) -> Arc<dyn TournamentRepository<RpCtx> + Send + Sync + 'static> {
        self.tournament_repository.clone()
    }
    fn tournament_player_repository(
        &self,
    ) -> Arc<dyn TournamentPlayerRepository<RpCtx> + Send + Sync + 'static> {
        self.tournament_player_repository.clone()
    }
    fn match_repository(&self) -> Arc<dyn MatchRepository<RpCtx> + Send + Sync + 'static> {
        self.match_repository.clone()
    }
    fn tournament_round_repository(
        &self,
    ) -> Arc<dyn TournamentRoundRepository<RpCtx> + Send + Sync + 'static> {
        self.tournament_round_repository.clone()
    }
    fn moderation_repository(
        &self,
    ) -> Arc<dyn ModerationRepository<RpCtx> + Send + Sync + 'static> {
        self.moderation_repository.clone()
    }
    fn game_state_repository(&self) -> Arc<dyn GameStateRepository<RpCtx> + Send + Sync + 'static> {
        self.game_state_repository.clone()
    }
    fn game_history_repository(
        &self,
    ) -> Arc<dyn GameHistoryRepository<RpCtx> + Send + Sync + 'static> {
        self.game_history_repository.clone()
    }
    fn transaction_manager(&self) -> Arc<dyn TransactionManager<RpCtx> + Send + Sync + 'static> {
        self.transaction_manager.clone()
    }

    fn email_port(&self) -> Arc<dyn EmailPort + Send + Sync + 'static> {
        self.email_port.clone()
    }
    fn notification_port(&self) -> Arc<dyn NotificationPort + Send + Sync + 'static> {
        self.notification_port.clone()
    }
    fn authentication_port(&self) -> Arc<dyn AuthenticationPort + Send + Sync + 'static> {
        self.authentication_port.clone()
    }
    fn user_online_status_port(&self) -> Arc<dyn UserOnlineStatusPort + Send + Sync + 'static> {
        self.user_online_status_port.clone()
    }

    fn seek_service(&self) -> Arc<dyn SeekService + Send + Sync + 'static> {
        self.seek_service.clone()
    }
    fn match_readiness_service(&self) -> Arc<dyn MatchReadinessService + Send + Sync + 'static> {
        self.match_readiness_service.clone()
    }
}

pub async fn build_application<TrCtx: Send + Sync + 'static>(
    game_repository: Arc<dyn GameRepository<TrCtx> + Send + Sync + 'static>,
    rating_repository: Arc<dyn RatingRepository<TrCtx> + Send + Sync + 'static>,
    event_repository: Arc<dyn EventRepository + Send + Sync + 'static>,
    stats_repository: Arc<dyn StatsRepository<TrCtx> + Send + Sync + 'static>,
    profile_repository: Arc<dyn ProfileRepository<TrCtx> + Send + Sync + 'static>,
    profile_picture_repository: Arc<dyn ProfilePictureRepository + Send + Sync + 'static>,
    chat_repository: Arc<dyn ChatRepository<TrCtx> + Send + Sync + 'static>,
    rating_history_repository: Arc<dyn RatingHistoryRepository<TrCtx> + Send + Sync + 'static>,
    tournament_repository: Arc<dyn TournamentRepository<TrCtx> + Send + Sync + 'static>,
    tournament_player_repository: Arc<
        dyn TournamentPlayerRepository<TrCtx> + Send + Sync + 'static,
    >,
    match_repository: Arc<dyn MatchRepository<TrCtx> + Send + Sync + 'static>,
    tournament_round_repository: Arc<dyn TournamentRoundRepository<TrCtx> + Send + Sync + 'static>,
    moderation_repository: Arc<dyn ModerationRepository<TrCtx> + Send + Sync + 'static>,
    game_state_repository: Arc<dyn GameStateRepository<TrCtx> + Send + Sync + 'static>,
    game_history_repository: Arc<dyn GameHistoryRepository<TrCtx> + Send + Sync + 'static>,
    transaction_manager: Arc<dyn TransactionManager<TrCtx> + Send + Sync + 'static>,

    email_port: Arc<dyn EmailPort + Send + Sync + 'static>,
    notification_port: Arc<dyn NotificationPort + Send + Sync + 'static>,
    authentication_port: Arc<dyn AuthenticationPort + Send + Sync + 'static>,
) -> Application {
    let ports = Arc::new(PortsCollectionImpl::new(
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
        email_port,
        notification_port,
        authentication_port,
        Arc::new(UserOnlineStatusService::new()),
        Arc::new(SeekServiceImpl::new()),
        Arc::new(MatchReadinessServiceImpl::new()),
    ));

    let game_timeout_scheduler = Arc::new(GameTimeoutRunnerImpl::new(ports.clone()));
    let player_disconnect_timeout_scheduler =
        Arc::new(DisconnectTimeoutRunnerImpl::new(ports.clone()));

    let application = Application {
        matchmaking_use_case: Arc::new(MatchmakingUseCaseImpl::new(
            ports.clone(),
            game_timeout_scheduler.clone(),
        )),
        player_use_case: Arc::new(PlayerUseCaseImpl::new(
            ports.clone(),
            player_disconnect_timeout_scheduler.clone(),
        )),
        gameplay_use_case: Arc::new(GameplayUseCaseImpl::new(ports.clone())),
        chat_use_case: Arc::new(ChatUseCaseImpl::new(ports.clone())),
        moderation_use_case: Arc::new(ModerationUseCaseImpl::new(ports.clone())),
        events_use_case: Arc::new(EventsUseCaseImpl::new(ports.clone())),
        profile_use_case: Arc::new(ProfileUseCaseImpl::new(ports.clone())),
        tournament_use_case: Arc::new(TournamentUseCaseImpl::new(ports.clone())),
        authentication_use_case: Arc::new(AuthenticationUseCaseImpl::new(ports.clone())),
        shutdown_use_case: Arc::new(ShutdownUseCaseImpl::new()),
    };

    application
}
