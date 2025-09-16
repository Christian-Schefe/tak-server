use crate::{
    client::ClientId,
    player::{try_login_guest, try_login_jwt},
};

pub fn handle_login_message(id: &ClientId, token: &str) {
    if let Err(e) = try_login_jwt(id, &token) {
        eprintln!("Login with token failed for user {}: {}", id, e);
    } else {
        println!("User {} logged in successfully", id);
    }
}

pub fn handle_login_guest_message(id: &ClientId, token: Option<&str>) {
    if let Err(e) = try_login_guest(id, token) {
        eprintln!("Guest login failed for user {}: {}", id, e);
    }
}
