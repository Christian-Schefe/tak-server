use futures_util::{SinkExt, StreamExt};
use tokio::{net::TcpStream, sync::mpsc::UnboundedSender};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

#[tokio::main]
async fn main() {
    let p1 = spawn_player().await;
    let p2 = spawn_player().await;

    p1.send("Login testuser pw".to_string()).unwrap();
    p2.send("Login testuser2 pw".to_string()).unwrap();

    p1.send("Seek 5 300 5 W 0 21 1 0 0 5 500".to_string())
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    p2.send("Accept 1".to_string()).unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    p1.send("Game#1 P A1 ".to_string()).unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    p2.send("Game#1 P A2 ".to_string()).unwrap();

    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
}

async fn create_connection() -> WebSocketStream<MaybeTlsStream<TcpStream>> {
    let (ws_stream, _) = connect_async("ws://localhost:9999")
        .await
        .expect("Failed to connect");
    ws_stream
}

async fn spawn_player() -> UnboundedSender<String> {
    let mut ws_stream = create_connection().await;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(msg) = rx.recv() => {
                    if ws_stream.send(Message::Text(msg.into())).await.is_err() {
                        break;
                    }
                }
                Some(Ok(msg)) = ws_stream.next() => {
                    println!("Received: {:?}", msg);
                }
                else => break,
            }
        }
    });
    tx
}
