use tak_client::playtak::{JsonSeek, PlaytakClient};

#[tokio::main]
async fn main() {
    let (client, handle) = PlaytakClient::new(
        "ws://localhost:9999",
        "http://localhost:9999",
        "testuser",
        "pw",
    );
    client
        .create_seek(JsonSeek {
            opponent: None,
            color: "random".to_string(),
            tournament: false,
            unrated: true,
            board_size: 5,
            half_komi: 0,
            reserve_pieces: 20,
            reserve_capstones: 1,
            time_ms: 300000,
            increment_ms: 10000,
            extra_move: None,
            extra_time_ms: None,
        })
        .await
        .unwrap();
    let _ = handle.await;
}
