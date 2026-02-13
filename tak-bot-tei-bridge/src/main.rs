use std::sync::Arc;

use clap::{Parser, command};
use tak_core::{
    TakBaseGameSettings, TakFinishedBaseGame, TakOngoingBaseGame, TakReserve,
    ptn::{action_from_ptn, action_to_ptn},
};
use tak_server_api::ClientMessage;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

use crate::server_api::ServerApi;

mod server_api;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    bot_executable: String,
}

struct App {
    bot_executable: String,
    server_api: Arc<ServerApi>,
}

struct EngineConnection {
    send_tei: tokio::sync::mpsc::UnboundedSender<String>,
    receive_tei: tokio::sync::mpsc::UnboundedReceiver<String>,
    cancellation_token: CancellationToken,
    game: Option<TakOngoingBaseGame>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    println!(
        "Starting TEI bridge with bot executable: {}",
        cli.bot_executable
    );

    let app = Arc::new(App {
        bot_executable: cli.bot_executable.clone(),
        server_api: ServerApi::new("ws://localhost:3003/ws", "https://localhost/api2"),
    });

    app.server_api
        .send_message(ClientMessage::Authenticate {
            token: "".to_string(),
        })
        .await
        .unwrap();

    let mut engine_connection = setup_engine(app.clone()).await;
    let settings = TakBaseGameSettings {
        board_size: 5,
        half_komi: 2,
        reserve: TakReserve::new(21, 1),
    };
    engine_connection.initialize(settings).await;

    loop {
        if let Some(best_move) = engine_connection.search_move(3000).await {
            println!("Best move received from engine: {}", best_move);
            if let Some(finished_game) = engine_connection.apply_move(&best_move) {
                let moves = finished_game
                    .action_history
                    .iter()
                    .map(|a| action_to_ptn(a))
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("Game finished! Moves: {}", moves);
                break;
            }
        } else {
            println!("No best move received from engine.");
            break;
        }
    }

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for shutdown signal");

    println!("Shutdown signal received. Shutting down TEI bridge...");
    engine_connection.shutdown().await;
    app.server_api.shutdown().await;
}

async fn setup_engine(app: Arc<App>) -> EngineConnection {
    let mut child = tokio::process::Command::new(&app.bot_executable)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn executable");

    let stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let (out_tx, out_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    let cancellation_token = CancellationToken::new();

    let cancellation_token_clone = cancellation_token.clone();
    let write_task = tokio::spawn(async move {
        let mut writer = tokio::io::BufWriter::new(stdin);
        while let Some(msg) = tokio::select! {
            msg = rx.recv() => msg,
            _ = cancellation_token_clone.cancelled() => None,
        } {
            let _ = writer.write_all(msg.as_bytes()).await;
            let _ = writer.write_all(b"\n").await;
            let _ = writer.flush().await;
        }
    });

    let cancellation_token_clone = cancellation_token.clone();
    let read_task = tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stdout);
        let mut line = String::new();
        while let Ok(_) = tokio::select! {
            res = reader.read_line(&mut line) => res,
            _ = cancellation_token_clone.cancelled() => return,
        } && !line.is_empty()
        {
            out_tx
                .send(line.trim().to_string())
                .expect("Failed to send bot output");
            line.clear();
        }
    });

    EngineConnection {
        send_tei: tx,
        receive_tei: out_rx,
        cancellation_token,
        game: None,
        tasks: vec![write_task, read_task],
    }
}

impl EngineConnection {
    async fn initialize(&mut self, settings: TakBaseGameSettings) {
        self.send_tei
            .send("tei".to_string())
            .expect("Failed to send TEI command");

        while let Some(output) = self.receive_tei.recv().await {
            if output == "teiok" {
                break;
            }
        }

        self.send_tei
            .send(format!("teinewgame {}", settings.board_size))
            .expect("Failed to send TEI new game command");

        self.game = Some(TakOngoingBaseGame::new(settings));
    }

    fn apply_move(&mut self, ptn_move: &str) -> Option<TakFinishedBaseGame> {
        let action = action_from_ptn(ptn_move).expect("Failed to parse PTN move");
        self.game
            .as_mut()
            .expect("Game not initialized")
            .do_action(action)
            .expect("Failed to apply action")
    }

    async fn search_move(&mut self, time_limit_ms: u32) -> Option<String> {
        let Some(game) = &self.game else {
            panic!("Game not initialized");
        };
        let moves = game
            .action_history
            .iter()
            .map(|a| action_to_ptn(a))
            .collect::<Vec<_>>()
            .join(" ");
        self.send_tei
            .send(format!("position startpos moves {}", moves))
            .expect("Failed to send TEI think command");

        self.send_tei
            .send(format!("go movetime {}", time_limit_ms))
            .expect("Failed to send TEI think command");

        while let Some(output) = self.receive_tei.recv().await {
            //println!("Bot output: {}", output);
            if output.starts_with("bestmove") {
                let best_move = output.trim_start_matches("bestmove ").to_string();
                println!("Best move: {}", best_move);
                return Some(best_move);
            }
        }
        None
    }

    async fn shutdown(mut self) {
        self.cancellation_token.cancel();
        for task in &mut self.tasks {
            let _ = task.await;
        }
    }
}
