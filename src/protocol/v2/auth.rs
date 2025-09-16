use crate::{
    client::{ClientId, send_to},
    player::{
        change_password, reset_password, send_reset_token, try_login, try_login_guest,
        try_login_jwt, try_register,
    },
};

pub fn handle_login_message(id: &ClientId, parts: &[&str]) {
    if parts.len() >= 2 && parts[1] == "Guest" {
        let token = parts.get(2).copied();
        match try_login_guest(id, token) {
            Ok(username) => {
                send_to(id, format!("Welcome {}!", username));
            }
            Err(e) => {
                println!("Guest login failed for user {}: {}", id, e);
                send_to(id, "NOK");
            }
        }
        return;
    }
    if parts.len() != 3 {
        send_to(id, "NOK");
    }
    let username = parts[1].to_string();
    let password = parts[2].to_string();

    if let Err(e) = try_login(id, &username, &password) {
        println!("Login failed for user {}: {}", id, e);
        send_to(id, "NOK");
    } else {
        send_to(id, format!("Welcome {}!", username));
    }
}

pub fn handle_login_token_message(id: &ClientId, parts: &[&str]) {
    if parts.len() != 2 {
        send_to(id, "NOK");
        return;
    }
    let token = parts[1];
    match try_login_jwt(id, token) {
        Ok(username) => {
            send_to(id, format!("Welcome {}!", username));
        }
        Err(e) => {
            println!("Login with token failed for user {}: {}", id, e);
            send_to(id, "NOK");
        }
    }
}

pub fn handle_register_message(id: &ClientId, parts: &[&str]) {
    if parts.len() != 3 {
        send_to(id, "NOK");
        return;
    }
    let username = parts[1].to_string();
    let email = parts[2].to_string();

    if let Err(e) = try_register(&username, &email) {
        println!("Error registering user {}: {}", username, e);
        send_to(id, format!("Registration Error: {}", e));
    } else {
        send_to(
            id,
            format!(
                "Registered {}. Check your email for the temporary password",
                username
            ),
        );
    }
}

pub fn handle_reset_token_message(id: &ClientId, parts: &[&str]) {
    if parts.len() != 3 {
        send_to(id, "NOK");
        return;
    }
    let username = parts[1].to_string();
    let email = parts[2].to_string();

    if let Err(e) = send_reset_token(&username, &email) {
        println!("Error sending reset token to user {}: {}", username, e);
        send_to(id, format!("Send Reset Token Error: {}", e));
    } else {
        send_to(id, "Reset token sent. Check your email for the token.");
    }
}

// TODO: handle passwords with spaces
pub fn handle_reset_password_message(id: &ClientId, parts: &[&str]) {
    if parts.len() != 4 {
        send_to(id, "NOK");
        return;
    }
    let username = parts[1].to_string();
    let token = parts[2].to_string();
    let new_password = parts[3].to_string();

    if let Err(e) = reset_password(&username, &token, &new_password) {
        println!("Error resetting password for client {}: {}", id, e);
        send_to(id, format!("Password Reset Error: {}", e));
    } else {
        send_to(
            id,
            "Password reset. Check your email for the temporary password.",
        );
    }
}

pub fn handle_change_password_message(id: &ClientId, parts: &[&str]) {
    if parts.len() != 3 {
        send_to(id, "NOK");
        return;
    }
    let old_password = parts[1].to_string();
    let new_password = parts[2].to_string();

    if let Err(e) = change_password(id, &old_password, &new_password) {
        println!("Error changing password for client {}: {}", id, e);
        send_to(id, format!("Change Password Error: {}", e));
    } else {
        send_to(id, "Password changed successfully.");
    }
}
