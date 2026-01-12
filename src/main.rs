use std::sync::Arc;

use log::info;

use tak_auth_ory::AuthenticationService;
use tak_email_lettre::LettreEmailAdapter;
use tak_events_google_sheets::NoopEventRepository;
use tak_persistence_sea_orm::{
    games::GameRepositoryImpl, player_account_mapping::PlayerAccountMappingRepositoryImpl,
    profile::ProfileRepositoryImpl, ratings::RatingRepositoryImpl,
};
use tak_player_connection::PlayerNotificationService;
use tak_server_api::WsService;
use tak_server_app::build_application;
use tak_server_legacy_api::{acl::LegacyAPIAntiCorruptionLayer, client::TransportServiceImpl};

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

    info!("Shutdown signal received. Preparing graceful exit...");
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
    init_logger();

    let legacy_transport_service = Arc::new(TransportServiceImpl::new());

    let ws_service = Arc::new(WsService::new());

    let player_notification_service = Arc::new(PlayerNotificationService::new(vec![
        legacy_transport_service.clone(),
        ws_service.clone(),
    ]));

    let game_repo = Arc::new(GameRepositoryImpl::new().await);
    let player_repo = Arc::new(PlayerAccountMappingRepositoryImpl::new().await);
    let rating_repo = Arc::new(RatingRepositoryImpl::new().await);
    let profile_repo = Arc::new(ProfileRepositoryImpl::new().await);
    let event_repo = Arc::new(NoopEventRepository);
    let email_adapter = Arc::new(LettreEmailAdapter::new());
    let player_connection_adapter = player_notification_service.clone();
    let listener_notification_adapter = Arc::new(ComposedListenerNotificationService::new(vec![
        player_connection_adapter.clone(), //for now only one adapter
    ]));
    let authentication_service = Arc::new(AuthenticationService::new());

    let app = Arc::new(
        build_application(
            game_repo,
            player_repo,
            rating_repo,
            event_repo,
            email_adapter.clone(),
            listener_notification_adapter,
            player_connection_adapter,
            authentication_service.clone(),
            profile_repo,
        )
        .await,
    );

    let acl = Arc::new(LegacyAPIAntiCorruptionLayer::new(
        app.clone(),
        authentication_service.clone(),
        email_adapter.clone(),
    ));

    info!("Starting application");

    let app_clone = app.clone();
    let auth_clone = authentication_service.clone();
    let acl_clone = acl.clone();
    let legacy_http_app = tokio::spawn(async move {
        tak_server_legacy_api::http::run(app_clone, auth_clone, acl_clone, shutdown_signal()).await;
    });

    let app_clone = app.clone();
    let auth_clone = authentication_service.clone();
    let http_app = tokio::spawn(async move {
        tak_server_api::serve(app_clone, auth_clone, ws_service, shutdown_signal()).await;
    });

    let transport_app = tokio::spawn(async move {
        TransportServiceImpl::run(
            legacy_transport_service,
            app,
            authentication_service.clone(),
            acl,
            shutdown_signal(),
        )
        .await;
    });

    let (r1, r2, r3) = tokio::join!(legacy_http_app, http_app, transport_app);

    if let Err(e) = r1 {
        log::error!("HTTP Legacy API task failed: {}", e);
    }

    if let Err(e) = r2 {
        log::error!("HTTP API task failed: {}", e);
    }

    if let Err(e) = r3 {
        log::error!("Transport service task failed: {}", e);
    }
}
