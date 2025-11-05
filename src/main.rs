mod app;
mod client;
mod jwt;
mod protocol;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Failed to load .env file");

    app::run().await;
}
