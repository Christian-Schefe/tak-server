use std::sync::{Arc, Mutex};

use tak_core::TakPlayer;
use tak_server_api::{
    IdentityInfo,
    game::{
        GameSettingsInfo, GameSettingsInfoBase, GameStatus, JsonGameMetadata, JsonTimeSettings,
    },
    seek::{CreateSeekPayload, SeekInfo},
    ws::{ClientMessage, ServerGameEventType, ServerMessage},
};
use tokio::{select, sync::mpsc::UnboundedReceiver};
use tokio_util::sync::CancellationToken;

use crate::{engine::EngineService, game::GameService, seek::SeekService};

#[async_trait::async_trait]
pub trait ServerApi {
    async fn send_message(&self, message: ClientMessage) -> Result<(), String>;
    async fn load_games(&self) -> Result<Vec<JsonGameMetadata>, String>;
    async fn load_game(&self, id: i64) -> Result<GameStatus, String>;
    async fn create_seek(&self, seek: CreateSeekPayload) -> Result<SeekInfo, String>;
}

pub struct Orchestrator {
    identity: IdentityInfo,
    server_api: Arc<dyn ServerApi + Send + Sync>,
    game_service: Arc<GameService>,
    seek_service: Arc<SeekService>,
    engine_service: Arc<EngineService>,
    handler_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    cancellation_token: CancellationToken,
}

fn get_seek_payload() -> CreateSeekPayload {
    CreateSeekPayload {
        opponent_id: None,
        color: "random".to_string(),
        is_rated: true,
        game_settings: GameSettingsInfo {
            base: GameSettingsInfoBase {
                board_size: 6,
                half_komi: 4,
                pieces: 30,
                capstones: 1,
            },
            time_settings: JsonTimeSettings::Realtime {
                increment_ms: 5_000,
                contingent_ms: 300_000,
                extra: None,
            },
        },
    }
}

impl Orchestrator {
    pub fn new(
        identity: IdentityInfo,
        server_api: Arc<dyn ServerApi + Send + Sync>,
        game_service: Arc<GameService>,
        seek_service: Arc<SeekService>,
        engine_service: Arc<EngineService>,
        rx: UnboundedReceiver<ServerMessage>,
    ) -> Arc<Self> {
        let this = Arc::new(Self {
            identity,
            server_api,
            game_service,
            seek_service,
            engine_service,
            handler_task: Arc::new(Mutex::new(None)),
            cancellation_token: CancellationToken::new(),
        });
        let this_clone = this.clone();
        let handler = tokio::spawn(async move {
            Self::run(this_clone, rx).await;
        });
        this.handler_task.lock().unwrap().replace(handler);
        this
    }

    fn get_player_color(&self, game: &JsonGameMetadata) -> Option<TakPlayer> {
        if game.player_ids.white == self.identity.player_id {
            Some(TakPlayer::White)
        } else if game.player_ids.black == self.identity.player_id {
            Some(TakPlayer::Black)
        } else {
            None
        }
    }

    pub async fn run(this: Arc<Self>, mut rx: UnboundedReceiver<ServerMessage>) {
        let games = this.server_api.load_games().await.unwrap();
        for game in &games {
            let Some(player) = this.get_player_color(game) else {
                continue;
            };
            let status = this.server_api.load_game(game.id).await.unwrap();
            if let Some((player, game_state)) =
                this.game_service.load_game(game.id, player, &status)
            {
                let time_remaining = match player {
                    TakPlayer::White => status.remaining_ms.white,
                    TakPlayer::Black => status.remaining_ms.black,
                };
                Self::on_my_turn(this.clone(), game.id, game_state, time_remaining);
            }
        }

        // TODO: Create seek sync that allows recovery if the seek disappears without the bot noticing.
        Self::create_seek(&this).await;

        while let Some(message) = select! {
            msg = rx.recv() => msg,
            _ = this.cancellation_token.cancelled() => None,
        } {
            println!("Received message: {:?}", message);
            match message {
                ServerMessage::Success { .. } => {}
                ServerMessage::Error { .. } => {}
                ServerMessage::SeekCreated { .. } => {}
                ServerMessage::SeekRemoved { seek_id } => {
                    if this.seek_service.end_seek(seek_id) {
                        println!("Our seek {} was removed, creating a new one", seek_id);
                        Self::create_seek(&this).await;
                    } else {
                        println!("Other player removed seek {}, ignoring", seek_id);
                    }
                }
                ServerMessage::GameEvent {
                    game_id,
                    event_type: event,
                    time_info,
                } => match event {
                    ServerGameEventType::GameAction { ply_index, action } => {
                        if let Some((player, game_state)) =
                            this.game_service.do_action(game_id, &action, ply_index)
                        {
                            let remaining_ms = match player {
                                TakPlayer::White => time_info.white,
                                TakPlayer::Black => time_info.black,
                            };
                            Self::on_my_turn(this.clone(), game_id, game_state, remaining_ms);
                        }
                    }
                    ServerGameEventType::GameActionUndone { ply_index } => {
                        if let Some((player, game_state)) =
                            this.game_service.undo_action(game_id, ply_index)
                        {
                            let remaining_ms = match player {
                                TakPlayer::White => time_info.white,
                                TakPlayer::Black => time_info.black,
                            };
                            Self::on_my_turn(this.clone(), game_id, game_state, remaining_ms);
                        }
                    }
                    ServerGameEventType::GameEnded { .. } => {}
                    ServerGameEventType::GameRequestAdded { .. } => {}
                    ServerGameEventType::GameRequestRemoved { .. } => {}
                },
                ServerMessage::GameStarted { game } => {
                    let Some(player) = this.get_player_color(&game) else {
                        continue;
                    };
                    let game_id = game.id;
                    let game_settings = game.game_settings.to_game_settings().base;
                    this.engine_service
                        .new_game(game_id, game_settings.clone())
                        .await;
                    if let Some((_, game_state)) =
                        this.game_service.begin_game(game_id, player, game_settings)
                    {
                        let time_remaining = match game.game_settings.time_settings {
                            JsonTimeSettings::Realtime { contingent_ms, .. } => contingent_ms,
                            JsonTimeSettings::Async { contingent_ms } => contingent_ms,
                        };

                        Self::on_my_turn(this.clone(), game_id, game_state, time_remaining);
                    }
                }
                ServerMessage::GameEnded { game_id } => {
                    this.game_service.end_game(game_id);
                    this.engine_service.remove_game(game_id).await;
                }
                ServerMessage::ChatMessage { .. } => {}
            }
        }
    }

    async fn create_seek(this: &Arc<Self>) {
        match this.server_api.create_seek(get_seek_payload()).await {
            Ok(seek) => {
                println!("Created seek: {}", seek.id);
                this.seek_service.begin_seek(seek.id);
            }
            Err(e) => {
                eprintln!("Failed to create seek: {}", e);
            }
        }
    }

    fn on_my_turn(
        this: Arc<Self>,
        game_id: i64,
        game_state: tak_core::TakOngoingBaseGame,
        remaining_ms: u64,
    ) {
        tokio::spawn(async move {
            let best_move = this
                .engine_service
                .search_move(game_id, &game_state, 3000.min(remaining_ms / 2))
                .await;
            if let Some(best_move) = best_move {
                if let Err(e) = this
                    .server_api
                    .send_message(ClientMessage::GameAction {
                        game_id,
                        action: best_move,
                    })
                    .await
                {
                    eprintln!("Failed to send move for game {}: {}", game_id, e);
                }
            } else {
                eprintln!("Engine failed to find a move for game {}", game_id);
            }
        });
    }

    pub async fn shutdown(&self) {
        self.cancellation_token.cancel();
        if let Some(handler) = self.handler_task.lock().unwrap().take() {
            let _ = handler.await;
        }
    }
}
