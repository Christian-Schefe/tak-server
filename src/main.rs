use std::sync::LazyLock;

use axum::{
    Router,
    extract::WebSocketUpgrade,
    response::Response,
    routing::{any, post},
};

mod app;
mod chat;
mod client;
mod email;
mod game;
mod jwt;
mod player;
mod protocol;
mod seek;
mod tak;

pub use app::*;

static APP: LazyLock<AppState> = LazyLock::new(construct_app);

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let app = Router::new()
        .route("/", any(ws_handler))
        .route("/ws", any(ws_handler))
        .route("/auth/login", post(jwt::handle_login));

    let app = APP.protocol_service.register_http_endpoints(app);

    let ws_port = std::env::var("TAK_WS_PORT")
        .unwrap_or_else(|_| "9999".to_string())
        .parse::<u16>()
        .expect("TAK_WS_PORT must be a valid u16");

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", ws_port))
        .await
        .unwrap();

    APP.player_service
        .load_unique_usernames()
        .expect("Failed to load unique usernames");

    tokio::spawn(async move {
        serve_tcp_server().await;
    });
    launch_background_tasks();

    println!("WebSocket server listening on port {}", ws_port);
    axum::serve(listener, app.with_state(APP.clone()))
        .await
        .unwrap();
}

fn launch_background_tasks() {
    tokio::spawn(async move {
        APP.client_service.launch_client_cleanup_task().await;
    });
}

async fn ws_handler(ws: WebSocketUpgrade) -> Response {
    ws.protocols(["binary"])
        .on_upgrade(move |socket| async move {
            APP.client_service.handle_client_websocket(socket).await;
        })
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
        tokio::spawn(async move {
            APP.client_service.handle_client_tcp(socket).await;
        });
    }
}
