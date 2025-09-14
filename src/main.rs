use axum::{
    Router,
    extract::{WebSocketUpgrade, ws::WebSocket},
    response::IntoResponse,
    routing::any,
};

use crate::{
    client::{handle_client_tcp, handle_client_websocket, launch_client_cleanup_task},
    email::send_email,
    player::load_unique_usernames,
};

mod chat;
mod client;
mod email;
mod game;
mod player;
mod protocol;
mod seek;
mod tak;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let app = Router::new()
        .route("/", any(ws_handler))
        .route("/ws", any(ws_handler));

    let ws_port = std::env::var("TAK_WS_PORT")
        .unwrap_or_else(|_| "9999".to_string())
        .parse::<u16>()
        .expect("TAK_WS_PORT must be a valid u16");

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", ws_port))
        .await
        .unwrap();

    load_unique_usernames().expect("Failed to load unique usernames");

    tokio::spawn(async move {
        serve_tcp_server().await;
    });
    launch_background_tasks();

    println!("WebSocket server listening on port {}", ws_port);
    axum::serve(listener, app).await.unwrap();
}

fn launch_background_tasks() {
    launch_client_cleanup_task();
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.protocols(["binary"])
        .on_upgrade(move |socket| handle_websocket(socket))
}

async fn handle_websocket(socket: WebSocket) {
    handle_client_websocket(socket);
}

async fn serve_tcp_server() {
    let tcp_port = std::env::var("TAK_TCP_PORT")
        .unwrap_or_else(|_| "10000".to_string())
        .parse::<u16>()
        .expect("TAK_TCP_PORT must be a valid u16");
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", tcp_port))
        .await
        .unwrap();
    println!("TCP server listening on port {}", tcp_port);
    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        println!("New TCP connection from {}", addr);
        handle_client_tcp(socket);
    }
}
