use futures::{SinkExt, StreamExt};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tak_server_api_contract::{
    auth::IdentityInfo,
    game::{GameStatus, JsonGameMetadata},
    seek::{CreateSeekPayload, SeekInfo},
    ws::{ClientMessage, ClientMessageWrapper, ServerMessage},
};
use tokio::{
    select,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};
use tokio_util::sync::CancellationToken;

use crate::orchestrator::ServerApi;

pub struct ServerApiImpl {
    ws_url: String,
    http_url: String,
    send_ws: UnboundedSender<WsSendInput>,
    cancellation_token: CancellationToken,
    ws_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    server_message_handler: UnboundedSender<ServerMessage>,
    http_client: reqwest::Client,
    auth_token: String,
}

type WsSendInput = (ClientMessage, UnboundedSender<Result<(), String>>);

impl ServerApiImpl {
    pub fn new(
        ws_url: &str,
        http_url: &str,
        server_message_handler: UnboundedSender<ServerMessage>,
        auth_token: String,
    ) -> Arc<Self> {
        let (ws_tx, ws_rx) = tokio::sync::mpsc::unbounded_channel();

        let cancellation_token = CancellationToken::new();
        let this = Arc::new(Self {
            ws_url: ws_url.to_string(),
            http_url: http_url.to_string(),
            send_ws: ws_tx,
            cancellation_token: cancellation_token.clone(),
            ws_task: Arc::new(Mutex::new(None)),
            server_message_handler,
            http_client: reqwest::Client::new(),
            auth_token: auth_token.clone(),
        });

        let this_clone = this.clone();
        let ws_task = tokio::spawn(async move {
            ServerApiImpl::run_ws(this_clone, ws_rx, auth_token, cancellation_token).await;
        });
        *this.ws_task.lock().unwrap() = Some(ws_task);
        this
    }

    pub async fn shutdown(&self) {
        self.cancellation_token.cancel();
        if let Some(handle) = self.ws_task.lock().unwrap().take() {
            let _ = handle.await;
        }
    }

    async fn handle_server_message(
        response_map: &Arc<Mutex<HashMap<uuid::Uuid, UnboundedSender<Result<(), String>>>>>,
        msg: ServerMessage,
        handler: &UnboundedSender<ServerMessage>,
    ) {
        println!("Received server message: {:?}", msg);
        match msg {
            ServerMessage::Success { response_id } => {
                if let Some(tx) = response_map.lock().unwrap().remove(&response_id) {
                    let _ = tx.send(Ok(()));
                }
            }
            ServerMessage::Error {
                message,
                code,
                response_id,
            } => {
                if let Some(tx) = response_map.lock().unwrap().remove(&response_id) {
                    let _ = tx.send(Err(format!("Error {}: {}", code, message)));
                }
            }
            msg => {
                if let Err(e) = handler.send(msg) {
                    eprintln!("Failed to send server message to handler: {}", e);
                }
            }
        }
    }

    async fn run_ws(
        api: Arc<ServerApiImpl>,
        mut ws_rx: UnboundedReceiver<WsSendInput>,
        auth_token: String,
        cancellation_token: CancellationToken,
    ) {
        loop {
            let ws_stream = match tokio_tungstenite::connect_async(&api.ws_url).await {
                Ok((ws_stream, _)) => {
                    println!("Connected to WebSocket at {}", api.ws_url);
                    ws_stream
                }
                Err(e) => {
                    eprintln!("Failed to connect to WebSocket: {}", e);
                    select! {
                        _ = cancellation_token.cancelled() => break,
                        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {},
                    }
                    eprintln!("Retrying WebSocket connection...");
                    continue;
                }
            };

            let response_map = Arc::new(Mutex::new(HashMap::new()));

            let (mut ws_write, mut ws_read) = ws_stream.split();

            let auth_message = ClientMessage::Authenticate {
                token: auth_token.to_string(),
            };
            let auth_message = serde_json::to_string(&ClientMessageWrapper {
                response_id: uuid::Uuid::new_v4(),
                message: auth_message,
            })
            .unwrap();
            if let Err(e) = ws_write
                .send(tokio_tungstenite::tungstenite::Message::Text(
                    auth_message.into(),
                ))
                .await
            {
                eprintln!("Failed to send authentication message: {}", e);
                select! {
                    _ = cancellation_token.cancelled() => break,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {},
                }
                eprintln!("Reconnecting to WebSocket...");
                continue;
            }

            let response_map_clone = response_map.clone();
            let child_cancellation_token = cancellation_token.child_token();
            let child_cancellation_token_clone = child_cancellation_token.clone();
            let handler_clone = api.server_message_handler.clone();
            let recv_task = tokio::spawn(async move {
                while let Some(msg) = select! {
                    _ = child_cancellation_token_clone.cancelled() => None,
                    msg = ws_read.next() => msg,
                } {
                    match msg {
                        Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                            if let Ok(response) = serde_json::from_str::<ServerMessage>(&text) {
                                Self::handle_server_message(
                                    &response_map_clone,
                                    response,
                                    &handler_clone,
                                )
                                .await;
                            } else {
                                eprintln!("Failed to parse server message: {}", text);
                            }
                        }
                        Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => {
                            eprintln!("WebSocket connection closed");
                            break;
                        }
                        Err(e) => {
                            eprintln!("WebSocket error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
                child_cancellation_token_clone.cancel();
            });

            while let Some((message, response_tx)) = select! {
                _ = child_cancellation_token.cancelled() => None,
                msg = ws_rx.recv() => msg,
            } {
                let response_id = uuid::Uuid::new_v4();
                let msg = serde_json::to_string(&ClientMessageWrapper {
                    response_id,
                    message,
                })
                .unwrap();
                let msg = tokio_tungstenite::tungstenite::Message::Text(msg.into());
                if let Err(e) = ws_write.send(msg).await {
                    eprintln!("Failed to send message: {}", e);
                    continue;
                }
                response_map
                    .lock()
                    .unwrap()
                    .insert(response_id, response_tx);
            }
            child_cancellation_token.cancel();
            recv_task.await.unwrap();
            select! {
                _ = cancellation_token.cancelled() => break,
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {},
            }
            eprintln!("Reconnecting to WebSocket...");
        }
    }

    pub async fn who_am_i(&self) -> Result<IdentityInfo, String> {
        self.get_request("/whoami?bot=true").await
    }

    async fn get_request<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
    ) -> Result<T, String> {
        let url = format!("{}{}", self.http_url, endpoint);
        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&self.auth_token)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "HTTP request failed with status: {}, body: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        response
            .json::<T>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    async fn post_request<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
        body: impl serde::ser::Serialize,
    ) -> Result<T, String> {
        let url = format!("{}{}", self.http_url, endpoint);
        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&self.auth_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "HTTP request failed with status: {}, body: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        response
            .json::<T>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }
}

#[async_trait::async_trait]
impl ServerApi for ServerApiImpl {
    async fn send_message(&self, message: ClientMessage) -> Result<(), String> {
        let (response_tx, mut response_rx) = tokio::sync::mpsc::unbounded_channel();
        self.send_ws
            .send((message, response_tx))
            .map_err(|_| "Failed to send message".to_string())?;
        response_rx
            .recv()
            .await
            .ok_or_else(|| "Failed to receive response".to_string())?
    }

    async fn load_games(&self) -> Result<Vec<JsonGameMetadata>, String> {
        self.get_request("/games").await
    }

    async fn load_game(&self, id: i64) -> Result<GameStatus, String> {
        self.get_request(&format!("/games/{}", id)).await
    }

    async fn create_seek(&self, seek: CreateSeekPayload) -> Result<SeekInfo, String> {
        self.post_request("/seeks", seek).await
    }
}
