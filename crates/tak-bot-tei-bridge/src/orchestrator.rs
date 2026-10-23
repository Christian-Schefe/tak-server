use std::sync::{Arc, Mutex};

use tak_core::{TakPlayer, TakReserve};

use tak_server_api_contract::{
    game::{
        JsonBaseGameSettings, JsonGameDetail, JsonGameMetadata, JsonGameSettings, JsonTakOpening,
        JsonTimeSettings,
    },
    seek::{CreateSeekPayload, JsonSeek},
    ws::{ClientMessage, ServerGameEventType, ServerMessage},
};
use tokio::{select, sync::mpsc::UnboundedReceiver};
use tokio_util::sync::CancellationToken;

use crate::{engine::EngineService, game::GameService, seek::SeekService};

#[async_trait::async_trait]
pub trait ServerApi {
    async fn send_message(&self, message: ClientMessage) -> Result<(), String>;
    async fn load_games(&self) -> Result<Vec<JsonGameMetadata>, String>;
    async fn load_game(&self, id: String) -> Result<JsonGameDetail, String>;
    async fn create_seek(&self, seek: CreateSeekPayload) -> Result<JsonSeek, String>;
}

pub struct Orchestrator {
    user_id: String,
    server_api: Arc<dyn ServerApi + Send + Sync>,
    game_service: Arc<GameService>,
    seek_service: Arc<SeekService>,
    engine_service: Arc<EngineService>,
    handler_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    cancellation_token: CancellationToken,
}

fn get_seek_payload() -> CreateSeekPayload {
    let reserve = TakReserve::from_size(6).unwrap();
    CreateSeekPayload {
        color: "random".to_string(),
        is_rated: true,
        game_settings: JsonGameSettings {
            base: JsonBaseGameSettings {
                board_size: 6,
                half_komi: 4,
                pieces: reserve.pieces,
                capstones: reserve.capstones,
                opening: JsonTakOpening::Swap,
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
        user_id: String,
        server_api: Arc<dyn ServerApi + Send + Sync>,
        game_service: Arc<GameService>,
        seek_service: Arc<SeekService>,
        engine_service: Arc<EngineService>,
        rx: UnboundedReceiver<ServerMessage>,
    ) -> Arc<Self> {
        let this = Arc::new(Self {
            user_id,
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
        if game.white.user_id == self.user_id {
            Some(TakPlayer::White)
        } else if game.black.user_id == self.user_id {
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
            let status = this.server_api.load_game(game.id.clone()).await.unwrap();
            let None = status.record.result else {
                continue;
            };
            if let Some((player, game_state)) =
                this.game_service
                    .load_game(game.id.clone(), player, &status)
            {
                let time_remaining = match player {
                    TakPlayer::White => status.remaining_ms.white,
                    TakPlayer::Black => status.remaining_ms.black,
                };
                Self::on_my_turn(this.clone(), game.id.clone(), game_state, time_remaining);
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
                    if this.seek_service.end_seek(seek_id.clone()) {
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
                            this.game_service
                                .do_action(game_id.clone(), &action, ply_index)
                        {
                            let remaining_ms = match player {
                                TakPlayer::White => time_info.white,
                                TakPlayer::Black => time_info.black,
                            };
                            Self::on_my_turn(
                                this.clone(),
                                game_id.clone(),
                                game_state,
                                remaining_ms,
                            );
                        }
                    }
                    ServerGameEventType::GameActionUndone { ply_index } => {
                        if let Some((player, game_state)) =
                            this.game_service.undo_action(game_id.clone(), ply_index)
                        {
                            let remaining_ms = match player {
                                TakPlayer::White => time_info.white,
                                TakPlayer::Black => time_info.black,
                            };
                            Self::on_my_turn(
                                this.clone(),
                                game_id.clone(),
                                game_state,
                                remaining_ms,
                            );
                        }
                    }
                    ServerGameEventType::GameEnded { .. } => {}
                    ServerGameEventType::GameRequestChanged { .. } => {}
                },
                ServerMessage::GameStarted { game } => {
                    let Some(player) = this.get_player_color(&game) else {
                        continue;
                    };
                    let game_id = game.id.clone();
                    let game_settings = game.game_settings.to_game_settings();
                    this.engine_service
                        .new_game(game_id.clone(), game_settings.base.clone())
                        .await;
                    if let Some((_, game_state)) =
                        this.game_service
                            .begin_game(game_id.clone(), player, &game_settings)
                    {
                        let time_remaining = match game.game_settings.time_settings {
                            JsonTimeSettings::Realtime { contingent_ms, .. } => contingent_ms,
                            JsonTimeSettings::Async { contingent_ms } => contingent_ms,
                        };

                        Self::on_my_turn(this.clone(), game_id.clone(), game_state, time_remaining);
                    }
                }
                ServerMessage::GameEnded { game_id } => {
                    this.game_service.end_game(game_id.clone());
                    this.engine_service.remove_game(game_id.clone()).await;
                }
                ServerMessage::ChatMessage { .. } => {}
                ServerMessage::MatchEvent { .. } => {}
                ServerMessage::UsersOnline { .. } => {}
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
        game_id: String,
        game_state: tak_core::TakGame<()>,
        remaining_ms: u64,
    ) {
        tokio::spawn(async move {
            let best_move = this
                .engine_service
                .search_move(
                    game_id.clone(),
                    game_state.iter_actions(),
                    3000.min(remaining_ms / 2),
                )
                .await;
            if let Some(best_move) = best_move {
                if let Err(e) = this
                    .server_api
                    .send_message(ClientMessage::GameAction {
                        game_id: game_id.clone(),
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
