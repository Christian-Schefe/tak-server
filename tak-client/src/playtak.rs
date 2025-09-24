use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::task::JoinHandle;

use crate::{SendError, TakClient};

#[derive(Clone)]
pub struct PlaytakClient {
    http_url: String,
    ws_client: TakClient,
    http_client: reqwest::Client,
    credentials: Arc<Mutex<Option<(String, String)>>>,
    token: Arc<Mutex<Option<String>>>,
    state: Arc<Mutex<PlaytakState>>,
}

pub struct PlaytakState {
    pub players: Vec<String>,
    pub seeks: HashMap<usize, JsonSeek>,
}

impl PlaytakState {
    pub fn new() -> Self {
        Self {
            players: vec![],
            seeks: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum JsonServerMessage {
    SeekList { add: bool, seek_id: usize },
    GameList { add: bool, game_id: usize },
    GameStart { game_id: usize },
    PlayersOnline { players: Vec<String> },
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ServerResponse {
    Ok,
    Error(String),
}

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("HTTP request failed")]
    Request(#[from] reqwest::Error),

    #[error("Failed to serialize/deserialize message")]
    Serde(#[from] serde_json::Error),

    #[error("Invalid response from server")]
    NotOkResponse,
}

#[derive(Serialize)]
pub struct GetTokenRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
pub struct GetTokenResponse {
    pub token: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonSeek {
    pub opponent: Option<String>,
    pub color: String,
    pub tournament: bool,
    pub unrated: bool,
    pub board_size: u32,
    pub half_komi: u32,
    pub reserve_pieces: u32,
    pub reserve_capstones: u32,
    pub time_ms: u64,
    pub increment_ms: u64,
    pub extra_move: Option<u32>,
    pub extra_time_ms: Option<u64>,
}

impl PlaytakClient {
    pub fn new(
        ws_url: impl Into<String>,
        http_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> (Self, JoinHandle<()>) {
        let ws_url = ws_url.into();
        let http_url = http_url.into();
        let token = Arc::new(Mutex::new(None));
        let http_client = reqwest::Client::new();
        let (ws_client, rx) = TakClient::new();
        let client = Self {
            ws_client: ws_client.clone(),
            http_client,
            http_url,
            token,
            credentials: Arc::new(Mutex::new(Some((username.into(), password.into())))),
            state: Arc::new(Mutex::new(PlaytakState::new())),
        };
        let client_clone = client.clone();
        let client_clone2 = client.clone();
        let handle = ws_client.run(
            rx,
            &ws_url,
            move || {
                let client_clone = client_clone.clone();
                Box::pin(async move {
                    match client_clone.login().await {
                        Ok(true) => println!("Logged in successfully"),
                        Ok(false) => println!("Login failed"),
                        Err(e) => println!("Login error: {:?}", e),
                    }
                })
            },
            move |msg| {
                let client_clone = client_clone2.clone();
                Box::pin(async move {
                    client_clone.handle_server_message(msg).await;
                })
            },
        );
        let client_clone = client.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                let resp = client_clone.send_ping().await;
                println!("Sent ping to server: {:?}", resp);
            }
        });
        (client, handle)
    }

    async fn handle_server_message(&self, msg: Value) {
        println!("Server message: {:?}", msg);
        if let Ok(server_msg) = serde_json::from_value::<JsonServerMessage>(msg.clone()) {
            match server_msg {
                JsonServerMessage::PlayersOnline { players } => {
                    let mut state = self.state.lock().unwrap();
                    state.players = players;
                }
                JsonServerMessage::SeekList { add, seek_id } => {
                    if add {
                        let seek = self.get_seek(seek_id as u64).await;
                        if let Ok(seek) = seek {
                            let mut state = self.state.lock().unwrap();
                            state.seeks.insert(seek_id, seek);
                        }
                    } else {
                        let mut state = self.state.lock().unwrap();
                        state.seeks.remove(&seek_id);
                    }
                }
                _ => {}
            }
        } else {
            println!("Failed to parse server message: {:?}", msg);
        }
    }

    pub fn with_state<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&PlaytakState) -> R,
    {
        let state = self.state.lock().unwrap();
        f(&state)
    }

    pub async fn request_token(
        &self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<String, HttpError> {
        let body = serde_json::to_string(&GetTokenRequest {
            username: username.into(),
            password: password.into(),
        })?;
        let resp = self
            .http_client
            .post(format!("{}/auth/login", self.http_url))
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(HttpError::NotOkResponse);
        }
        let json = resp.text().await?;
        let json: GetTokenResponse = serde_json::from_str(&json)?;
        let mut token = self.token.lock().unwrap();
        *token = Some(json.token.clone());
        Ok(json.token)
    }

    fn get_token(&self) -> Option<String> {
        let token = self.token.lock().unwrap();
        token.clone()
    }

    async fn get_or_retrieve_token(&self) -> Option<String> {
        if let Some(token) = self.get_token() {
            //TODO: validate expiry
            return Some(token.clone());
        }
        let Some((username, password)) = self.credentials.lock().unwrap().clone() else {
            return None;
        };
        self.request_token(username, password).await.ok()
    }

    pub async fn login(&self) -> Result<bool, SendError> {
        let token = self.get_or_retrieve_token().await;
        if token.is_none() {
            return Ok(false);
        }
        let msg = serde_json::json!({
            "type": "login",
            "token": token.as_ref().unwrap()
        });
        let resp = self.ws_client.send::<Value, ServerResponse>(msg).await?;
        match resp {
            ServerResponse::Ok => Ok(true),
            _ => Ok(false),
        }
    }

    pub async fn send_ping(&self) -> Result<bool, SendError> {
        let msg = serde_json::json!({ "type": "ping" });
        let resp = self.ws_client.send::<Value, ServerResponse>(msg).await?;
        match resp {
            ServerResponse::Ok => Ok(true),
            _ => Ok(false),
        }
    }

    pub async fn create_seek(&self, seek: JsonSeek) -> Result<(), HttpError> {
        let token = self.get_or_retrieve_token().await;
        if token.is_none() {
            return Err(HttpError::NotOkResponse);
        }
        let body = serde_json::to_string(&seek)?;
        let resp = self
            .http_client
            .post(format!("{}/v3/seeks", self.http_url))
            .header("Content-Type", "application/json")
            .bearer_auth(token.as_ref().unwrap())
            .body(body)
            .send()
            .await?;
        if !resp.status().is_success() {
            println!("Response: {:?}", resp.text().await);
            return Err(HttpError::NotOkResponse);
        }
        Ok(())
    }

    pub async fn remove_seek(&self) -> Result<(), HttpError> {
        let token = self.get_or_retrieve_token().await;
        if token.is_none() {
            return Err(HttpError::NotOkResponse);
        }
        let resp = self
            .http_client
            .delete(format!("{}/v3/seeks", self.http_url))
            .bearer_auth(token.as_ref().unwrap())
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(HttpError::NotOkResponse);
        }
        Ok(())
    }

    pub async fn get_seek(&self, seek_id: u64) -> Result<JsonSeek, HttpError> {
        let token = self.get_or_retrieve_token().await;
        if token.is_none() {
            return Err(HttpError::NotOkResponse);
        }
        let resp = self
            .http_client
            .get(format!("{}/v3/seeks/{}", self.http_url, seek_id))
            .bearer_auth(token.as_ref().unwrap())
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(HttpError::NotOkResponse);
        }
        let json = resp.text().await?;
        let seek: JsonSeek = serde_json::from_str(&json)?;
        Ok(seek)
    }

    pub async fn accept_seek(&self, seek_id: u64) -> Result<(), HttpError> {
        let token = self.get_or_retrieve_token().await;
        if token.is_none() {
            return Err(HttpError::NotOkResponse);
        }
        let resp = self
            .http_client
            .post(format!("{}/v3/seeks/{}/accept", self.http_url, seek_id))
            .bearer_auth(token.as_ref().unwrap())
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(HttpError::NotOkResponse);
        }
        Ok(())
    }
}
