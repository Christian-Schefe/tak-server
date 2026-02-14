use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tak_core::{TakBaseGameSettings, TakOngoingBaseGame, ptn::action_to_ptn};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

pub struct EngineService {
    bot_executable: String,
    engine_connections: Arc<Mutex<HashMap<i64, EngineConnection>>>,
}

impl EngineService {
    pub fn new(bot_executable: String) -> Self {
        Self {
            bot_executable,
            engine_connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn new_game(&self, game_id: i64, settings: TakBaseGameSettings) {
        let mut connection = EngineConnection::new(&self.bot_executable).await;
        connection.initialize(settings).await;
        self.engine_connections
            .lock()
            .unwrap()
            .insert(game_id, connection);
    }

    pub async fn remove_game(&self, game_id: i64) {
        let res = { self.engine_connections.lock().unwrap().remove(&game_id) };
        if let Some(connection) = res {
            connection.shutdown().await;
        }
    }

    pub async fn search_move(
        &self,
        game_id: i64,
        game: &TakOngoingBaseGame,
        time_limit_ms: u64,
    ) -> Option<String> {
        let res = { self.engine_connections.lock().unwrap().remove(&game_id) };
        if let Some(mut connection) = res {
            let result = connection.search_move(game, time_limit_ms).await;
            self.engine_connections
                .lock()
                .unwrap()
                .insert(game_id, connection);
            result
        } else {
            None
        }
    }
}

pub struct EngineConnection {
    send_tei: tokio::sync::mpsc::UnboundedSender<String>,
    receive_tei: tokio::sync::mpsc::UnboundedReceiver<String>,
    cancellation_token: CancellationToken,
    tasks: Vec<tokio::task::JoinHandle<()>>,
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
    }

    async fn search_move(
        &mut self,
        game: &TakOngoingBaseGame,
        time_limit_ms: u64,
    ) -> Option<String> {
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

    pub async fn new(executable: &str) -> EngineConnection {
        let mut child = tokio::process::Command::new(&executable)
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
            tasks: vec![write_task, read_task],
        }
    }
}
