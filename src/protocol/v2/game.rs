use crate::{
    client::{ClientId, get_associated_player, send_to},
    game::{GameId, offer_draw, request_undo, resign_game, try_do_action},
    player::PlayerUsername,
    protocol::ServerGameMessage,
    tak::{TakAction, TakDir, TakGameState, TakPos, TakVariant},
};

pub fn handle_game_server_message(id: &ClientId, game_id: &GameId, msg: &ServerGameMessage) {
    match msg {
        ServerGameMessage::Action(action) => {
            send_game_action_message(id, game_id, action);
        }
        ServerGameMessage::GameOver(game_state) => {
            send_game_over_message(id, game_id, game_state);
        }
        ServerGameMessage::DrawOffer { offer } => {
            let message = format!(
                "Game#{} {}",
                game_id,
                if *offer { "OfferDraw" } else { "RemoveDraw" }
            );
            send_to(id, message);
        }
        ServerGameMessage::UndoRequest { request } => {
            let message = format!(
                "Game#{} {}",
                game_id,
                if *request {
                    "RequestUndo"
                } else {
                    "RemoveUndo"
                }
            );
            send_to(id, message);
        }
        ServerGameMessage::Undo => {
            let message = format!("Game#{} Undo", game_id);
            send_to(id, message);
        }
        ServerGameMessage::TimeUpdate { remaining } => {
            let message = format!(
                "Game#{} Timems {} {}",
                game_id,
                remaining.0.as_millis(),
                remaining.1.as_millis()
            );
            send_to(id, message);
        }
    }
}

pub fn send_game_over_message(id: &ClientId, game_id: &GameId, game_state: &TakGameState) {
    if *game_state == TakGameState::Ongoing {
        return;
    }
    let message = format!("Game#{} Over {}", game_id, game_state.to_string());
    send_to(id, message);
}

pub fn handle_game_message(id: &ClientId, parts: &[&str]) {
    if parts.len() < 2 {
        println!("Invalid Game message format: {:?}", parts);
        send_to(id, "NOK");
        return;
    }
    let Some(username) = get_associated_player(id) else {
        println!("Client {} not associated with any player", id);
        send_to(id, "NOK");
        return;
    };
    let Some(Ok(game_id)) = parts[0].split("#").nth(1).map(|s| s.parse::<u32>()) else {
        println!("Invalid Game ID in Game message: {}", parts[1]);
        send_to(id, "NOK");
        return;
    };

    let result = match parts[1] {
        "P" => handle_game_place_message(&username, game_id, &parts[2..]),
        "M" => handle_game_move_message(&username, game_id, &parts[2..]),
        "Resign" => resign_game(&username, &game_id),
        "OfferDraw" => offer_draw(&username, &game_id, true),
        "RemoveDraw" => offer_draw(&username, &game_id, false),
        "RequestUndo" => request_undo(&username, &game_id, true),
        "RemoveUndo" => request_undo(&username, &game_id, false),
        _ => {
            println!("Unknown Game action: {}", parts[2]);
            Err("Unknown Game action".into())
        }
    };

    if let Err(e) = result {
        println!("Error handling Game message: {}", e);
        send_to(id, "NOK");
    }
}

pub fn handle_game_place_message(
    username: &PlayerUsername,
    game_id: u32,
    parts: &[&str],
) -> Result<(), String> {
    if parts.len() != 1 && parts.len() != 2 {
        return Err("Invalid Game Place message format".into());
    }
    let square = parts[0].chars().collect::<Vec<_>>();
    if square.len() != 2 {
        return Err("Invalid square format".into());
    }
    let x = (square[0] as u8).wrapping_sub(b'A') as u32;
    let y = (square[1] as u8).wrapping_sub(b'1') as u32;
    let variant = if parts.len() != 2 {
        TakVariant::Flat
    } else {
        match parts[1] {
            "W" => TakVariant::Standing,
            "C" => TakVariant::Capstone,
            _ => return Err("Invalid piece variant".into()),
        }
    };

    let action = TakAction::Place {
        pos: TakPos::new(x as i32, y as i32),
        variant,
    };

    try_do_action(username, &game_id, action)?;

    Ok(())
}

pub fn handle_game_move_message(
    username: &PlayerUsername,
    game_id: u32,
    parts: &[&str],
) -> Result<(), String> {
    if parts.len() < 3 {
        return Err("Invalid Game Move message format".into());
    }
    let from_square = parts[0].chars().collect::<Vec<_>>();
    if from_square.len() != 2 {
        return Err("Invalid square format".into());
    }
    let from_x = (from_square[0] as u8).wrapping_sub(b'A') as u32;
    let from_y = (from_square[1] as u8).wrapping_sub(b'1') as u32;
    let to_square = parts[1].chars().collect::<Vec<_>>();
    if to_square.len() != 2 {
        return Err("Invalid square format".into());
    }
    let to_x = (to_square[0] as u8).wrapping_sub(b'A') as u32;
    let to_y = (to_square[1] as u8).wrapping_sub(b'1') as u32;
    let dir = if from_x == to_x {
        if to_y > from_y {
            TakDir::Up
        } else {
            TakDir::Down
        }
    } else if from_y == to_y {
        if to_x > from_x {
            TakDir::Right
        } else {
            TakDir::Left
        }
    } else {
        return Err("Invalid move direction".into());
    };
    let drops = parts[2..]
        .iter()
        .map(|s| {
            s.parse::<u32>()
                .map_err(|_| "Invalid drop count".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let action = TakAction::Move {
        pos: TakPos::new(from_x as i32, from_y as i32),
        dir,
        drops,
    };
    try_do_action(username, &game_id, action)?;

    Ok(())
}

pub fn send_game_action_message(id: &ClientId, game_id: &GameId, action: &TakAction) {
    let message = match action {
        TakAction::Place { pos, variant } => format!(
            "Game#{} P {}{} {}",
            game_id,
            (b'A' + pos.x as u8) as char,
            (b'1' + pos.y as u8) as char,
            match variant {
                TakVariant::Flat => "",
                TakVariant::Standing => "W",
                TakVariant::Capstone => "C",
            }
        ),
        TakAction::Move { pos, dir, drops } => {
            let end_pos = pos.offset(dir, drops.len() as i32);
            let drops_str = drops
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                "Game#{} M {}{} {}{} {}",
                game_id,
                (b'A' + pos.x as u8) as char,
                (b'1' + pos.y as u8) as char,
                (b'A' + end_pos.x as u8) as char,
                (b'1' + end_pos.y as u8) as char,
                drops_str
            )
        }
    };
    send_to(&id, message);
}
