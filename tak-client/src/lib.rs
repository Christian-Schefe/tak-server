use std::{sync::Arc, time::Duration};

use dashmap::DashMap;
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    net::TcpStream,
    sync::{
        Mutex,
        mpsc::{UnboundedReceiver, UnboundedSender},
    },
    task::JoinHandle,
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub mod playtak;

#[derive(Clone)]
pub struct TakClient {
    tx: ClientSender,
    cancellation_token: Arc<Mutex<CancellationToken>>,
}

pub type SendListener = (
    serde_json::Value,
    tokio::sync::oneshot::Sender<serde_json::Value>,
);
type OpenMessageMap = Arc<DashMap<String, tokio::sync::oneshot::Sender<serde_json::Value>>>;
type ClientSender = Arc<UnboundedSender<SendListener>>;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrackedMessage {
    msg_id: String,
    #[serde(flatten)]
    message: serde_json::Value,
}

#[derive(Debug, Error)]
pub enum SendError {
    #[error("Failed to send message")]
    Send(#[from] tokio::sync::mpsc::error::SendError<SendListener>),
    #[error("Failed to receive response")]
    Recv(#[from] tokio::sync::oneshot::error::RecvError),
    #[error("Failed to serialize/deserialize message")]
    Serde(#[from] serde_json::Error),
    #[error("Response timed out")]
    Timeout,
}

impl TakClient {
    pub fn new() -> (Self, UnboundedReceiver<SendListener>) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<SendListener>();
        let cancellation_token = Arc::new(Mutex::new(CancellationToken::new()));
        let client = Self {
            tx: Arc::new(tx),
            cancellation_token: cancellation_token.clone(),
        };
        (client, rx)
    }

    pub fn run<F, F2>(
        &self,
        rx: UnboundedReceiver<SendListener>,
        url: impl Into<String>,
        on_connect: F,
        on_message: F2,
    ) -> JoinHandle<()>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + 'static,
        F2: Fn(serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
            + Send
            + Sync
            + Clone
            + 'static,
    {
        let url = url.into();
        let cancellation_token = self.cancellation_token.clone();
        let handle = tokio::spawn(async move {
            let mut cur_rx = rx;
            loop {
                let Ok(ws) = tokio_tungstenite::connect_async(&url).await else {
                    println!("Failed to connect to {}", url);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                };
                let token = CancellationToken::new();
                let mut token_lock = cancellation_token.lock().await;
                token_lock.cancel();
                *token_lock = token.clone();
                drop(token_lock);
                let (write, read) = ws.0.split();
                let open_messages = Arc::new(DashMap::new());
                let write_handle = tokio::spawn(handle_send(
                    write,
                    open_messages.clone(),
                    cur_rx,
                    token.clone(),
                ));
                let read_handle = tokio::spawn(handle_receive(
                    read,
                    open_messages,
                    token,
                    on_message.clone(),
                ));
                println!("Connected to {}", url);
                on_connect().await;
                let (rx, _) = tokio::join!(write_handle, read_handle);
                cur_rx = rx.unwrap();
                tokio::time::sleep(Duration::from_secs(1)).await;
                println!("Reconnecting to {}", url);
            }
        });
        handle
    }

    pub fn send_sync(
        &self,
        msg: serde_json::Value,
    ) -> Result<tokio::sync::oneshot::Receiver<serde_json::Value>, SendError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx.send((msg, tx))?;
        Ok(rx)
    }

    pub async fn send_json(&self, msg: serde_json::Value) -> Result<serde_json::Value, SendError> {
        let timeout = Duration::from_secs(10);
        let resp = tokio::select! {
            resp = self.send_sync(msg)? => resp?,
            _ = tokio::time::sleep(timeout) => Err(SendError::Timeout)?
        };
        Ok(resp)
    }

    pub async fn send<T, R>(&self, msg: T) -> Result<R, SendError>
    where
        T: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let msg = serde_json::to_value(msg)?;
        let resp = self.send_json(msg).await?;
        Ok(serde_json::from_value(resp)?)
    }

    pub async fn close(&self) {
        let token_lock = self.cancellation_token.lock().await;
        token_lock.cancel();
    }
}

async fn handle_send(
    mut write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    open_messages: OpenMessageMap,
    mut rx: UnboundedReceiver<SendListener>,
    cancellation_token: CancellationToken,
) -> UnboundedReceiver<SendListener> {
    write
        .send(Message::Text("Protocol 3".into()))
        .await
        .unwrap();

    while let Some((msg, listener)) = tokio::select! {
        msg = rx.recv() => msg,
        _ = cancellation_token.cancelled() => None,
    } {
        let id: String = Uuid::new_v4().to_string();
        let msg = serde_json::to_string(&TrackedMessage {
            msg_id: id.clone(),
            message: msg,
        })
        .unwrap();
        open_messages.insert(id, listener);
        write.send(Message::Text(msg.into())).await.unwrap();
    }
    cancellation_token.cancel();
    rx
}

async fn handle_receive(
    mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    open_messages: OpenMessageMap,
    cancellation_token: CancellationToken,
    on_message: impl Fn(
        serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
    + Send
    + 'static,
) {
    while let Some(msg) = tokio::select! {
        msg = read.next() => msg,
        _ = cancellation_token.cancelled() => None,
    } {
        match msg {
            Ok(Message::Binary(data)) => {
                println!("Received binary message: {:?}", data);
                if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&data) {
                    if let Ok(resp) = serde_json::from_value::<TrackedMessage>(value.clone()) {
                        if let Some(tx) = open_messages.remove(&resp.msg_id).map(|e| e.1) {
                            let _ = tx.send(resp.message);
                        } else {
                            println!("No listener for message id {}", resp.msg_id);
                        }
                    } else {
                        (on_message)(value).await;
                    }
                } else {
                    println!("Failed to parse binary message as JSON");
                }
            }
            Ok(Message::Close(_)) => {
                println!("Connection closed");
                break;
            }
            Ok(_) => {
                println!("Received non-binary message");
            }
            Err(e) => {
                println!("Error receiving message: {}", e);
                break;
            }
        }
    }
    cancellation_token.cancel();
}
