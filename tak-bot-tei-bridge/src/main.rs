use std::sync::Arc;

use clap::{Parser, command};

use crate::{
    game::GameService, orchestrator::Orchestrator, seek::SeekService, server_api::ServerApiImpl,
};

mod engine;
mod game;
mod orchestrator;
mod seek;
mod server_api;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    bot_executable: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    println!(
        "Starting TEI bridge with bot executable: {}",
        cli.bot_executable
    );

    let (server_msg_tx, server_msg_rx) = tokio::sync::mpsc::unbounded_channel();

    let game_service = Arc::new(GameService::new());
    let seek_service = Arc::new(SeekService::new());
    let engine_service = Arc::new(engine::EngineService::new(cli.bot_executable));
    let server_api = ServerApiImpl::new(
        "ws://localhost:3003/ws",
        "http://localhost:3003",
        server_msg_tx,
        "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIyYmFiZmY5ZC1lNzI2LTQ4N2UtODc4MS1iNzU3ZjBmNzM1NjAiLCJleHAiOjE3ODE3NjI2NzV9.k821BtChfIV9suKkMKPOl5px-CuFPtcER9pUVv4HtOY".to_string(),
    );
    let identity = server_api.who_am_i().await.expect("Failed to authenticate");
    println!("Authenticated as {:?}", identity);
    let orchestrator = Orchestrator::new(
        identity,
        server_api.clone(),
        game_service,
        seek_service,
        engine_service,
        server_msg_rx,
    );

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for shutdown signal");

    println!("Shutdown signal received. Shutting down TEI bridge...");
    orchestrator.shutdown().await;
    server_api.shutdown().await;
}
