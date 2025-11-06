use std::sync::Arc;

use tak_persistence_sqlite::{games::SqliteGameRepository, players::SqlitePlayerRepository};
use tak_server_api::{JwtServiceImpl, TransportServiceImpl};
use tak_server_domain::{
    app::{LazyAppState, construct_app},
    game::ArcGameRepository,
    jwt::ArcJwtService,
    player::ArcPlayerRepository,
    transport::ArcTransportService,
};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Failed to load .env file");

    let app = LazyAppState::new();
    let transport_service_impl = TransportServiceImpl::new(app.clone());

    let game_repo: ArcGameRepository = Arc::new(Box::new(SqliteGameRepository::new()));
    let player_repo: ArcPlayerRepository = Arc::new(Box::new(SqlitePlayerRepository::new()));
    let transport_service: ArcTransportService = Arc::new(Box::new(transport_service_impl.clone()));

    let jwt_service: ArcJwtService = Arc::new(Box::new(JwtServiceImpl {}));
    construct_app(
        app.clone(),
        game_repo,
        player_repo,
        jwt_service,
        transport_service,
    );

    transport_service_impl.run(app.clone()).await;
}
