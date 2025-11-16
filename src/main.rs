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
use tak_events_persistence::NoopEventRepository;
use tak_persistence_sea_orm::{games::GameRepositoryImpl, players::PlayerRepositoryImpl};
use tak_server_api::{JwtServiceImpl, TransportServiceImpl};
use tak_server_domain::{
    app::{LazyAppState, construct_app},
    event::ArcEventRepository,
    game::ArcGameRepository,
    jwt::ArcJwtService,
    player::ArcPlayerRepository,
    transport::ArcTransportService,
};

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

    let app = LazyAppState::new();
    let transport_service_impl = TransportServiceImpl::new(app.clone());

    let game_repo: ArcGameRepository = Arc::new(Box::new(GameRepositoryImpl::new().await));
    let player_repo: ArcPlayerRepository = Arc::new(Box::new(PlayerRepositoryImpl::new().await));
    let events_repo: ArcEventRepository = Arc::new(Box::new(NoopEventRepository {}));

    let transport_service: ArcTransportService = Arc::new(Box::new(transport_service_impl.clone()));
    let jwt_service: ArcJwtService = Arc::new(Box::new(JwtServiceImpl {}));

    construct_app(
        app.clone(),
        game_repo,
        player_repo,
        events_repo,
        jwt_service,
        transport_service,
    );

    info!("Starting application");

    let app_clone = app.clone();
    let http_app = tokio::spawn(async move {
        tak_server_http_api::run(app_clone, shutdown_signal()).await;
    });

    let transport_app = tokio::spawn(async move {
        transport_service_impl.run(app, shutdown_signal()).await;
    });

    let (r1, r2) = tokio::join!(http_app, transport_app);

    if let Err(e) = r1 {
        eprintln!("HTTP API task failed: {}", e);
    }

    if let Err(e) = r2 {
        eprintln!("Transport service task failed: {}", e);
    }
}
