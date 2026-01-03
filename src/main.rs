use std::sync::Arc;

use log::{LevelFilter, info};
use log4rs::{
    Config,
    append::{
        console::{ConsoleAppender, Target},
        rolling_file::policy::compound::{
            CompoundPolicy, roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger,
        },
    },
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    filter::threshold::ThresholdFilter,
};
use tak_auth_ory::OryAuthenticationService;
use tak_email_lettre::LettreEmailAdapter;
use tak_events_google_sheets::NoopEventRepository;
use tak_persistence_sea_orm::{
    games::GameRepositoryImpl, player_account_mapping::PlayerAccountMappingRepositoryImpl,
    profile::ProfileRepositoryImpl, ratings::RatingRepositoryImpl,
};
use tak_server_api::{acl::LegacyAPIAntiCorruptionLayer, client::TransportServiceImpl};
use tak_server_app::build_application;

const LOG_SIZE_LIMIT: u64 = 10 * 1024 * 1024; // 10 MB

const LOG_FILE_COUNT: u32 = 3;

fn init_logger() {
    let file_path = std::env::var("LOG_FILE_PATH").expect("LOG_FILE_PATH must be set");
    let archive_pattern =
        std::env::var("LOG_ARCHIVE_PATTERN").expect("LOG_ARCHIVE_PATTERN must be set");

    let stderr_level = LevelFilter::Info;
    let file_level = LevelFilter::Debug;

    let stderr = ConsoleAppender::builder().target(Target::Stderr).build();

    let trigger = SizeTrigger::new(LOG_SIZE_LIMIT);
    let roller = FixedWindowRoller::builder()
        .build(&archive_pattern, LOG_FILE_COUNT)
        .unwrap();
    let policy = CompoundPolicy::new(Box::new(trigger), Box::new(roller));

    let logfile = log4rs::append::rolling_file::RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build(file_path, Box::new(policy))
        .unwrap();

    let config = Config::builder()
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(file_level)))
                .build("logfile", Box::new(logfile)),
        )
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(stderr_level)))
                .build("stderr", Box::new(stderr)),
        )
        .build(
            Root::builder()
                .appender("logfile")
                .appender("stderr")
                .build(LevelFilter::Trace),
        )
        .unwrap();

    let _handle = log4rs::init_config(config).expect("Failed to initialize logger");
}

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

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Failed to load .env file");

    init_logger();

    let transport_service_impl = Arc::new(TransportServiceImpl::new());

    let game_repo = Arc::new(GameRepositoryImpl::new().await);
    let player_repo = Arc::new(PlayerAccountMappingRepositoryImpl::new().await);
    let rating_repo = Arc::new(RatingRepositoryImpl::new().await);
    let profile_repo = Arc::new(ProfileRepositoryImpl::new().await);
    let event_repo = Arc::new(NoopEventRepository);
    let email_adapter = Arc::new(LettreEmailAdapter::new());
    let player_connection_adapter = transport_service_impl.clone();
    let listener_notification_adapter = transport_service_impl.clone();
    let authentication_service = Arc::new(OryAuthenticationService::new());

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
    let http_app = tokio::spawn(async move {
        tak_server_api::http::run(app_clone, auth_clone, acl_clone, shutdown_signal()).await;
    });

    let transport_app = tokio::spawn(async move {
        TransportServiceImpl::run(
            transport_service_impl,
            app,
            authentication_service.clone(),
            acl,
            shutdown_signal(),
        )
        .await;
    });

    let (r1, r2) = tokio::join!(http_app, transport_app);

    if let Err(e) = r1 {
        log::error!("HTTP API task failed: {}", e);
    }

    if let Err(e) = r2 {
        log::error!("Transport service task failed: {}", e);
    }
}
