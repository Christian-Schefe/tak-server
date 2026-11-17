#![recursion_limit = "256"]

use std::sync::Arc;

use tak_auth::AuthService;
use tak_email_lettre::LettreEmailAdapter;
use tak_events_google_sheets::NoopEventRepository;
use tak_persistence_profile_pictures::ProfilePictureRepositoryImpl;
use tak_persistence_sea_orm::{
    account::AccountRepositoryImpl,
    chat::ChatRepositoryImpl,
    game_history::GameHistoryRepositoryImpl,
    game_state::GameStateRepositoryImpl,
    games::GameRepositoryImpl,
    matches::MatchRepositoryImpl,
    moderation::ModerationRepositoryImpl,
    profile::ProfileRepositoryImpl,
    rating_history::RatingHistoryRepositoryImpl,
    ratings::RatingRepositoryImpl,
    stats::StatsRepositoryImpl,
    tournament::{
        TournamentPlayerRegistrationRepositoryImpl, TournamentRepositoryImpl,
        TournamentRoundRepositoryImpl,
    },
    transaction::TransactionManagerImpl,
};
use tak_player_connection::{
    AccountOnlineStatusService, PlayerConnectionDriver, PlayerConnectionService,
};
use tak_server_api::WsService;
use tak_server_app::{PortsCollectionImpl, build_application};

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

    let game_repo = Arc::new(GameRepositoryImpl);
    let rating_repo = Arc::new(RatingRepositoryImpl);
    let profile_repo = Arc::new(ProfileRepositoryImpl);
    let event_repo = Arc::new(NoopEventRepository);
    let stats_repo = Arc::new(StatsRepositoryImpl);
    let email_adapter = Arc::new(LettreEmailAdapter::new());
    let player_connection_adapter =
        Arc::new(PlayerConnectionService::new(vec![ws_service.clone()]));
    let listener_notification_adapter = Arc::new(ComposedListenerNotificationService::new(vec![
        player_connection_adapter.clone(), //for now only one adapter
    ]));

    let account_repo = Arc::new(AccountRepositoryImpl::new().await);
    let authentication_adapter = Arc::new(AuthService::new(account_repo));

    let account_online_status_adapter = Arc::new(AccountOnlineStatusService::new());
    let profile_picture_repo = Arc::new(ProfilePictureRepositoryImpl::new().await);
    let chat_repo = Arc::new(ChatRepositoryImpl);
    let rating_history_repo = Arc::new(RatingHistoryRepositoryImpl);
    let tournament_repo = Arc::new(TournamentRepositoryImpl);
    let tournament_player_registration_repo = Arc::new(TournamentPlayerRegistrationRepositoryImpl);
    let match_repo = Arc::new(MatchRepositoryImpl);
    let tournament_round_repo = Arc::new(TournamentRoundRepositoryImpl);
    let moderation_repo = Arc::new(ModerationRepositoryImpl);
    let game_state_repo = Arc::new(GameStateRepositoryImpl);
    let game_history_repo = Arc::new(GameHistoryRepositoryImpl);

    let transaction_manager = Arc::new(TransactionManagerImpl::new().await);

    let ports = Arc::new(PortsCollectionImpl::new(
        game_repo,
        rating_repo,
        event_repo,
        stats_repo,
        profile_repo,
        profile_picture_repo,
        chat_repo,
        rating_history_repo,
        tournament_repo,
        tournament_player_registration_repo,
        match_repo,
        tournament_round_repo,
        moderation_repo,
        game_state_repo,
        game_history_repo,
        transaction_manager,
    ));
    let app = Arc::new(
        build_application(
            email_adapter.clone(),
            listener_notification_adapter.clone(),
            player_connection_adapter.clone(),
            authentication_adapter.clone(),
            account_online_status_adapter,
            ports,
        )
        .await,
    );

    let connection_driver = Arc::new(PlayerConnectionDriver::new(
        app.clone(),
        player_connection_adapter.clone(),
    ));

    tracing::info!("Starting application");

    let app_clone = app.clone();
    let on_shutdown = async move {
        shutdown_signal().await;
        tracing::info!("Executing shutdown workflow");
        app_clone.shutdown_use_case.shutdown().await;
    };

    tak_server_api::serve(app, ws_service, connection_driver, on_shutdown).await;
}
