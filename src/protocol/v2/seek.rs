use std::time::Duration;

use crate::{
    client::{ClientId, get_associated_player, send_to},
    player::fetch_player,
    seek::{GameType, Seek, accept_seek, add_seek, remove_seek_of_player},
    tak::{TakGameSettings, TakPlayer, TakTimeControl},
};

pub fn handle_seek_message(id: &ClientId, parts: &[&str]) {
    if let Err(e) = handle_add_seek_message(id, parts) {
        println!("Error parsing Seek message: {}", e);
        send_to(id, "NOK");
    }
}

pub fn handle_add_seek_message(id: &ClientId, parts: &[&str]) -> Result<(), String> {
    if parts.len() != 13 && parts.len() != 12 && parts.len() != 11 && parts.len() != 10 {
        println!("Invalid Seek message format: {:?}", parts);
        return Err("Invalid Seek message format".into());
    }
    let Some(username) = get_associated_player(id) else {
        println!("Client {} not associated with any player", id);
        return Err("Client not associated with any player".into());
    };
    let board_size = parts[1].parse::<u32>().map_err(|_| "Invalid board size")?;
    let time_contingent_seconds = parts[2]
        .parse::<u32>()
        .map_err(|_| "Invalid time contingent")?;
    let time_increment_seconds = parts[3]
        .parse::<u32>()
        .map_err(|_| "Invalid time increment")?;
    let color = match parts[4] {
        "W" => Some(crate::tak::TakPlayer::White),
        "B" => Some(crate::tak::TakPlayer::Black),
        "A" => None,
        _ => return Err("Invalid color".into()),
    };
    let half_komi = parts[5].parse::<u32>().map_err(|_| "Invalid half komi")?;
    let reserve_pieces = parts[6]
        .parse::<u32>()
        .map_err(|_| "Invalid reserve pieces")?;
    let reserve_capstones = parts[7]
        .parse::<u32>()
        .map_err(|_| "Invalid reserve capstones")?;
    let is_unrated = match parts[8] {
        "1" => true,
        "0" => false,
        _ => return Err("Invalid rated/unrated flag".into()),
    };
    let is_tournament = match parts[9] {
        "1" => true,
        "0" => false,
        _ => return Err("Invalid tournament flag".into()),
    };
    let game_type = match (is_unrated, is_tournament) {
        (true, false) => crate::seek::GameType::Unrated,
        (false, false) => crate::seek::GameType::Rated,
        (_, true) => crate::seek::GameType::Tournament,
    };
    let time_extra_trigger_move = if parts.len() >= 12 {
        parts[10]
            .parse::<u32>()
            .map_err(|_| "Invalid time extra trigger move")?
    } else {
        0
    };
    let time_extra_trigger_seconds = if parts.len() >= 12 {
        parts[11]
            .parse::<u32>()
            .map_err(|_| "Invalid time extra trigger seconds")?
    } else {
        0
    };
    let opponent = if parts.len() == 13 {
        Some(parts[12].to_string())
    } else if parts.len() == 11 {
        Some(parts[9].to_string())
    } else {
        None
    };

    let time_extra = if time_extra_trigger_move > 0 && time_extra_trigger_seconds > 0 {
        Some((
            time_extra_trigger_move,
            Duration::from_secs(time_extra_trigger_seconds as u64),
        ))
    } else {
        None
    };

    let game_settings = TakGameSettings {
        board_size,
        half_komi,
        reserve_pieces,
        reserve_capstones,
        time_control: TakTimeControl {
            contingent: Duration::from_secs(time_contingent_seconds as u64),
            increment: Duration::from_secs(time_increment_seconds as u64),
            extra: time_extra,
        },
    };

    if !game_settings.is_valid() {
        remove_seek_of_player(&username)?;
    } else {
        add_seek(username, opponent, color, game_settings, game_type)?;
    }

    Ok(())
}

pub fn handle_accept_message(id: &ClientId, parts: &[&str]) {
    if parts.len() != 2 {
        println!("Invalid Accept message format: {:?}", parts);
        send_to(id, "NOK");
        return;
    }
    let Ok(seek_id) = parts[1].parse::<u32>() else {
        println!("Invalid Seek ID in Accept message: {}", parts[1]);
        send_to(id, "NOK");
        return;
    };
    let Some(username) = get_associated_player(id) else {
        println!("Client {} not associated with any player", id);
        send_to(id, "NOK");
        return;
    };
    if let Err(e) = accept_seek(&seek_id, &username) {
        println!("Error accepting seek {}: {}", seek_id, e);
        send_to(id, "NOK");
        return;
    };
}

pub fn send_seek_list_message(id: &ClientId, seek: &Seek, add: bool) {
    let message = format!(
        "Seek {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
        if add { "new" } else { "remove" },
        seek.id,
        seek.creator,
        seek.game_settings.board_size,
        seek.game_settings.time_control.contingent.as_secs(),
        seek.game_settings.time_control.increment.as_secs(),
        seek.color.as_ref().map_or("A", |c| match c {
            TakPlayer::White => "W",
            TakPlayer::Black => "B",
        }),
        seek.game_settings.half_komi,
        seek.game_settings.reserve_pieces,
        seek.game_settings.reserve_capstones,
        match seek.game_type {
            GameType::Unrated => "1",
            GameType::Rated => "0",
            GameType::Tournament => "0",
        },
        match seek.game_type {
            GameType::Unrated => "0",
            GameType::Rated => "0",
            GameType::Tournament => "1",
        },
        seek.game_settings
            .time_control
            .extra
            .as_ref()
            .map_or("0".to_string(), |(trigger_move, _)| trigger_move
                .to_string()),
        seek.game_settings
            .time_control
            .extra
            .as_ref()
            .map_or("0".to_string(), |(_, extra_time)| extra_time
                .as_secs()
                .to_string()),
        seek.opponent.as_deref().unwrap_or(""),
        fetch_player(&seek.creator).map_or("0", |p| if p.is_bot { "1" } else { "0" })
    );
    send_to(id, message);
}
