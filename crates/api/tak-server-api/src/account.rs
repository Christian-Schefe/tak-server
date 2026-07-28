use axum::{Json, extract::State};

use crate::AppState;

pub fn register_routes() -> axum::Router<AppState> {
    axum::Router::new().route("/online", axum::routing::get(get_online_accounts))
}

async fn get_online_accounts(State(app): State<AppState>) -> Json<Vec<String>> {
    let accounts = app.app.account_get_online_use_case.get_online_accounts();
    Json(accounts.into_iter().map(|a| a.to_string()).collect())
}
