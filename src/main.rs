use std::sync::Arc;

use tak_auth_ory::AuthenticationService;
use tak_bot_registry::FileBotRepository;
use tak_email_lettre::LettreEmailAdapter;
use tak_events_google_sheets::NoopEventRepository;
use tak_persistence_profile_pictures::ProfilePictureRepositoryImpl;
use tak_persistence_sea_orm::{
    chat::ChatRepositoryImpl,
    games::GameRepositoryImpl,
    guest::GuestRepositoryImpl,
    matches::MatchRepositoryImpl,
    player_account_mapping::PlayerAccountMappingRepositoryImpl,
    profile::ProfileRepositoryImpl,
    puzzle::PuzzleRepositoryImpl,
    rating_history::RatingHistoryRepositoryImpl,
    ratings::RatingRepositoryImpl,
    stats::StatsRepositoryImpl,
    tournament::{
        TournamentPlayerRegistrationRepositoryImpl, TournamentRepositoryImpl,
        TournamentRoundRepositoryImpl,
    },
};
use tak_player_connection::{
    AccountOnlineStatusService, PlayerConnectionDriver, PlayerConnectionService,
};
use tak_server_api::WsService;
use tak_server_app::build_application;

use crate::{compose::ComposedListenerNotificationService, logs::init_logger};

mod compose;
mod logs;

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received. Preparing graceful exit...");
}

fn try_load_env() {
    let environment = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
    if environment == "production" {
        return;
    };
    let path_str = format!("deploy/.env");
    let env_path = std::path::Path::new(&path_str);

    dotenvy::from_path_override(env_path).expect("Failed to load environment variables from file");
    println!("Loaded environment variables from {}", env_path.display());
}

#[tokio::main]
async fn main() {
    try_load_env();
    let _guard = init_logger();
    let ws_service = Arc::new(WsService::new());

    let game_repo = Arc::new(GameRepositoryImpl::new().await);
    let player_repo = Arc::new(PlayerAccountMappingRepositoryImpl::new().await);
    let rating_repo = Arc::new(RatingRepositoryImpl::new().await);
    let profile_repo = Arc::new(ProfileRepositoryImpl::new().await);
    let event_repo = Arc::new(NoopEventRepository);
    let stats_repo = Arc::new(StatsRepositoryImpl::new().await);
    let email_adapter = Arc::new(LettreEmailAdapter::new());
    let player_connection_adapter =
        Arc::new(PlayerConnectionService::new(vec![ws_service.clone()]));
    let listener_notification_adapter = Arc::new(ComposedListenerNotificationService::new(vec![
        player_connection_adapter.clone(), //for now only one adapter
    ]));

    let bot_repository = Arc::new(FileBotRepository::new());
    let guest_repo = Arc::new(GuestRepositoryImpl::new().await);
    let authentication_adapter = Arc::new(AuthenticationService::new(bot_repository, guest_repo));

    let account_online_status_adapter = Arc::new(AccountOnlineStatusService::new());
    let profile_picture_repo = Arc::new(ProfilePictureRepositoryImpl::new().await);
    let puzzle_repo = Arc::new(PuzzleRepositoryImpl::new().await);
    let chat_repo = Arc::new(ChatRepositoryImpl::new().await);
    let rating_history_repo = Arc::new(RatingHistoryRepositoryImpl::new().await);
    let tournament_repo = Arc::new(TournamentRepositoryImpl::new().await);
    let tournament_player_registration_repo =
        Arc::new(TournamentPlayerRegistrationRepositoryImpl::new().await);
    let match_repo = Arc::new(MatchRepositoryImpl::new().await);
    let tournament_round_repo = Arc::new(TournamentRoundRepositoryImpl::new().await);

    let app = Arc::new(
        build_application(
            game_repo,
            player_repo,
            rating_repo,
            event_repo,
            stats_repo,
            email_adapter.clone(),
            listener_notification_adapter.clone(),
            player_connection_adapter.clone(),
            authentication_adapter.clone(),
            profile_repo,
            account_online_status_adapter,
            profile_picture_repo,
            puzzle_repo,
            chat_repo,
            rating_history_repo,
            tournament_repo,
            tournament_player_registration_repo,
            match_repo,
            tournament_round_repo,
        )
        .await,
    );

    let connection_driver = Arc::new(PlayerConnectionDriver::new(
        app.clone(),
        player_connection_adapter.clone(),
    ));

    tracing::info!("Starting application");

    tak_server_api::serve(
        app,
        authentication_adapter,
        ws_service,
        connection_driver,
        shutdown_signal(),
    )
    .await;
}
