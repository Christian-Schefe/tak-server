use crate::{
    client::{ClientId, send_to},
    player::{set_admin, set_banned, set_bot, set_gagged, set_modded},
};

pub fn handle_sudo_message(id: &ClientId, parts: &[&str]) {
    if parts.len() < 2 {
        eprintln!("Sudo command requires at least one argument");
        send_to(id, "NOK");
        return;
    }
    let command = parts[1];
    match command {
        "gag" => handle_player_update(id, parts, Some(true), None, None, None, None),
        "ungag" => handle_player_update(id, parts, Some(false), None, None, None, None),
        // TODO: ban message
        "ban" => handle_player_update(id, parts, None, Some(true), None, None, None),
        "unban" => handle_player_update(id, parts, None, Some(false), None, None, None),
        "mod" => handle_player_update(id, parts, None, None, Some(true), None, None),
        "unmod" => handle_player_update(id, parts, None, None, Some(false), None, None),
        "admin" => handle_player_update(id, parts, None, None, None, Some(true), None),
        "unadmin" => handle_player_update(id, parts, None, None, None, Some(false), None),
        "bot" => handle_player_update(id, parts, None, None, None, None, Some(true)),
        "unbot" => handle_player_update(id, parts, None, None, None, None, Some(false)),
        // TODO: more sudo commands
        "kick" => {}
        "list" => {}
        "reload" => {}
        "broadcast" => {}
        "set" => {}
        _ => {
            eprintln!("Unknown Sudo command: {}", command);
        }
    }
}

pub fn handle_player_update(
    id: &ClientId,
    parts: &[&str],
    gagged: Option<bool>,
    banned: Option<bool>,
    modded: Option<bool>,
    admin: Option<bool>,
    bot: Option<bool>,
) {
    if parts.len() != 3 {
        eprintln!("Invalid Sudo {} command format: {:?}", parts[1], parts);
        return;
    }
    let username = parts[2].to_string();
    if let Some(gagged) = gagged {
        if let Err(e) = set_gagged(id, &username, gagged) {
            eprintln!(
                "Failed to set gagged={} for user {}: {}",
                gagged, username, e
            );
        }
    }
    if let Some(banned) = banned {
        if let Err(e) = set_banned(id, &username, banned) {
            eprintln!(
                "Failed to set banned={} for user {}: {}",
                banned, username, e
            );
        }
    }
    if let Some(modded) = modded {
        if let Err(e) = set_modded(id, &username, modded) {
            eprintln!(
                "Failed to set modded={} for user {}: {}",
                modded, username, e
            );
        }
    }
    if let Some(admin) = admin {
        if let Err(e) = set_admin(id, &username, admin) {
            eprintln!("Failed to set admin={} for user {}: {}", admin, username, e);
        }
    }
    if let Some(bot) = bot {
        if let Err(e) = set_bot(id, &username, bot) {
            eprintln!("Failed to set bot={} for user {}: {}", bot, username, e);
        }
    }
}
