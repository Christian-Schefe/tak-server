use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tak_core::{TakBaseGameSettings, TakOngoingBaseGame, ptn::action_to_ptn};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    select,
};
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

struct TeiOption {
    name: String,
    option_type: TeiOptionType,
}

#[allow(dead_code)]
enum TeiOptionType {
    Spin {
        default: i64,
        min: i64,
        max: i64,
    },
    Combo {
        default: String,
        options: Vec<String>,
    },
}

impl TeiOption {
    fn parse_option(output: &str) -> Option<TeiOption> {
        let mut parts = HashMap::new();
        let words = output.split_whitespace().collect::<Vec<_>>();
        let mut key = None;
        for word in words {
            if word == "option" {
                continue;
            }
            if key.is_none() {
                key = Some(word.to_string());
            } else {
                parts.insert(key.take().unwrap(), word.to_string());
            }
        }
        let Some(name) = parts.get("name") else {
            return None;
        };
        let Some(option_type) = parts.get("type") else {
            return None;
        };
        let option_type = match option_type.as_str() {
            "spin" => {
                let default = parts.get("default")?.parse::<i64>().ok()?;
                let min = parts.get("min")?.parse::<i64>().ok()?;
                let max = parts.get("max")?.parse::<i64>().ok()?;
                TeiOptionType::Spin { default, min, max }
            }
            "combo" => {
                let default = parts.get("default")?.clone();
                let options = parts
                    .iter()
                    .filter(|(k, _)| k.starts_with("var"))
                    .map(|(_, v)| v.clone())
                    .collect::<Vec<_>>();
                TeiOptionType::Combo { default, options }
            }
            _ => return None,
        };
        let opt = TeiOption {
            name: name.clone(),
            option_type,
        };
        Some(opt)
    }

    fn is_valid_value(&self, value: &str) -> bool {
        match &self.option_type {
            TeiOptionType::Spin { min, max, .. } => {
                if let Ok(val) = value.parse::<i64>() {
                    return val >= *min && val <= *max;
                }
                false
            }
            TeiOptionType::Combo { options, .. } => options.contains(&value.to_string()),
        }
    }

    fn set_value_command(&self, value: &str) -> Option<String> {
        if self.is_valid_value(value) {
            Some(format!("setoption name {} value {}", self.name, value))
        } else {
            None
        }
    }
}

impl EngineConnection {
    async fn initialize(&mut self, settings: TakBaseGameSettings) {
        self.send_tei
            .send("tei".to_string())
            .expect("Failed to send TEI command");

        let mut options = HashMap::new();

        while let Some(output) = self.receive_tei.recv().await {
            if output.starts_with("option") {
                if let Some(opt) = TeiOption::parse_option(&output) {
                    options.insert(opt.name.to_ascii_lowercase().clone(), opt);
                }
            }
            if output == "teiok" {
                break;
            }
        }

        if let Some(komi_option) = options.get("halfkomi") {
            if let Some(cmd) = komi_option.set_value_command(&settings.half_komi.to_string()) {
                self.send_tei
                    .send(cmd)
                    .expect("Failed to send TEI setoption command");
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

        while let Some(output) = select! {
            output = self.receive_tei.recv() => output,
            _ = self.cancellation_token.cancelled() => None,
        } {
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
                println!("Bot output: {}", line.trim());
                if let Err(e) = out_tx.send(line.trim().to_string()) {
                    eprintln!("Failed to send TEI output: {}", e);
                }
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
