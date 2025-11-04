mod app;
mod client;
mod jwt;
mod persistence;
mod protocol;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    app::run().await;
}
