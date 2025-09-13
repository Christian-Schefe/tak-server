use std::time::{Duration, Instant};

use crate::{
    client::{ClientId, get_associated_player, try_send_to},
    game::{
        Game, GameId, observe_game, offer_draw, request_undo, resign_game, try_do_action,
        unobserve_game,
    },
    player::{PlayerUsername, fetch_player, login_guest, try_login},
    protocol::ServerMessage,
    seek::{GameType, Seek, accept_seek, add_seek},
    tak::{
        TakAction, TakDir, TakGameSettings, TakGameState, TakPlayer, TakPos, TakTimeControl,
        TakVariant,
    },
};

pub fn handle_client_message(id: &ClientId, msg: String) {
    let parts = msg.split_whitespace().collect::<Vec<_>>();
    if parts.is_empty() {
        println!("Received empty message");
        return;
    }
    match parts[0] {
        "Login" => handle_login_message(*id, &parts),
        "Seek" => handle_seek_message(id, &parts),
        "Accept" => handle_accept_message(id, &parts),
        "Observe" => handle_observe_message(id, &parts, false),
        "Unobserve" => handle_observe_message(id, &parts, true),
        "PING" => {
            let _ = try_send_to(id, "OK");
        }
        s if s.starts_with("Game#") => handle_game_message(id, &parts),
        _ => {
            println!("Unknown V2 message type: {}", parts[0]);
            let _ = try_send_to(id, "NOK");
        }
    };
}

pub fn handle_server_message(id: &ClientId, msg: &ServerMessage) {
    match msg {
        ServerMessage::SeekList { add, seek } => {
            send_seek_list_message(id, &seek, if *add { "new" } else { "remove" });
        }
        ServerMessage::GameList { add, game } => {
            send_game_string_message(
                id,
                game,
                if *add {
                    "GameList Add"
                } else {
                    "GameList Remove"
                },
            );
        }
        ServerMessage::GameAction { game_id, action } => {
            send_game_action_message(id, game_id, action);
        }
        ServerMessage::GameOver {
            game_id,
            game_state,
        } => {
            send_game_over_message(id, game_id, game_state);
        }
        ServerMessage::GameStart { game } => {
            send_game_start_message(id, game);
        }
        ServerMessage::PlayersOnline { players } => {
            let online_message = format!("Online {}", players.len());
            let players_message =
                format!("OnlinePlayers {}", serde_json::to_string(players).unwrap());
            let _ = try_send_to(id, online_message);
            let _ = try_send_to(id, players_message);
        }
        ServerMessage::ObserveGame { game } => {
            send_game_string_message(id, game, "Observe");
            for action in game.game.move_history.iter() {
                send_game_action_message(id, &game.id, action);
            }
            let now = Instant::now();
            let time_message = format!(
                "Game#{} Timems {} {}",
                game.id,
                game.game
                    .get_time_remaining(&TakPlayer::White, now)
                    .as_millis(),
                game.game
                    .get_time_remaining(&TakPlayer::Black, now)
                    .as_millis()
            );
            let _ = try_send_to(id, time_message);
        }
        ServerMessage::GameDrawOffer { game_id, offer } => {
            let message = format!(
                "Game#{} {}",
                game_id,
                if *offer { "OfferDraw" } else { "RemoveDraw" }
            );
            let _ = try_send_to(id, message);
        }
        ServerMessage::GameUndoRequest { game_id, request } => {
            let message = format!(
                "Game#{} {}",
                game_id,
                if *request {
                    "RequestUndo"
                } else {
                    "RemoveUndo"
                }
            );
            let _ = try_send_to(id, message);
        }
        ServerMessage::GameUndo { game_id } => {
            let message = format!("Game#{} Undo", game_id);
            let _ = try_send_to(id, message);
        }
        ServerMessage::GameTimeUpdate { game_id, remaining } => {
            let message = format!(
                "Game#{} Timems {} {}",
                game_id,
                remaining.0.as_millis(),
                remaining.1.as_millis()
            );
            let _ = try_send_to(id, message);
        }
    }
}

fn send_game_over_message(id: &ClientId, game_id: &GameId, game_state: &TakGameState) {
    if *game_state == TakGameState::Ongoing {
        return;
    }
    let message = format!("Game#{} Over {}", game_id, game_state.to_string());
    let _ = try_send_to(id, message);
}

fn handle_login_message(id: ClientId, parts: &[&str]) {
    if parts.len() >= 2 && parts[1] == "Guest" {
        let token = parts.get(2).copied();
        login_guest(&id, token);
        return;
    }
    if parts.len() != 3 {
        let _ = try_send_to(&id, "NOK");
    }
    let username = parts[1].to_string();
    let password = parts[2].to_string();

    if !try_login(&id, &username, &password) {
        println!("Login failed for user: {}", id);
        let _ = try_send_to(&id, "NOK");
    } else {
        let _ = try_send_to(&id, format!("Welcome {}!", username));
    }
}

fn handle_seek_message(id: &ClientId, parts: &[&str]) {
    if let Err(e) = handle_add_seek_message(id, parts) {
        println!("Error parsing Seek message: {}", e);
        let _ = try_send_to(id, "NOK");
    }
}

fn handle_add_seek_message(id: &ClientId, parts: &[&str]) -> Result<(), String> {
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

    add_seek(username, opponent, color, game_settings, game_type)?;

    Ok(())
}

fn handle_accept_message(id: &ClientId, parts: &[&str]) {
    if parts.len() != 2 {
        println!("Invalid Accept message format: {:?}", parts);
        let _ = try_send_to(id, "NOK");
        return;
    }
    let Ok(seek_id) = parts[1].parse::<u32>() else {
        println!("Invalid Seek ID in Accept message: {}", parts[1]);
        let _ = try_send_to(id, "NOK");
        return;
    };
    let Some(username) = get_associated_player(id) else {
        println!("Client {} not associated with any player", id);
        let _ = try_send_to(id, "NOK");
        return;
    };
    if let Err(e) = accept_seek(&seek_id, &username) {
        println!("Error accepting seek {}: {}", seek_id, e);
        let _ = try_send_to(id, "NOK");
        return;
    };
}

fn handle_observe_message(id: &ClientId, parts: &[&str], unobserve: bool) {
    if parts.len() != 2 {
        println!("Invalid Observe message format: {:?}", parts);
        let _ = try_send_to(id, "NOK");
        return;
    }
    let Ok(game_id) = parts[1].parse::<u32>() else {
        println!("Invalid Game ID in Observe message: {}", parts[1]);
        let _ = try_send_to(id, "NOK");
        return;
    };
    let Some(_) = get_associated_player(id) else {
        println!("Client {} not associated with any player", id);
        let _ = try_send_to(id, "NOK");
        return;
    };
    if unobserve {
        if let Err(e) = unobserve_game(id, &game_id) {
            println!("Error unobserving game {}: {}", game_id, e);
            let _ = try_send_to(id, "NOK");
            return;
        };
    } else {
        if let Err(e) = observe_game(id, &game_id) {
            println!("Error observing game {}: {}", game_id, e);
            let _ = try_send_to(id, "NOK");
            return;
        };
    }
}

fn handle_game_message(id: &ClientId, parts: &[&str]) {
    if parts.len() < 2 {
        println!("Invalid Game message format: {:?}", parts);
        let _ = try_send_to(id, "NOK");
        return;
    }
    let Some(username) = get_associated_player(id) else {
        println!("Client {} not associated with any player", id);
        let _ = try_send_to(id, "NOK");
        return;
    };
    let Some(Ok(game_id)) = parts[0].split("#").nth(1).map(|s| s.parse::<u32>()) else {
        println!("Invalid Game ID in Game message: {}", parts[1]);
        let _ = try_send_to(id, "NOK");
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
        let _ = try_send_to(id, "NOK");
    }
}

fn handle_game_place_message(
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

fn handle_game_move_message(
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

fn send_seek_list_message(id: &ClientId, seek: &Seek, operation: &str) {
    let message = format!(
        "Seek {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
        operation,
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
    let _ = try_send_to(id, message);
}

fn send_game_string_message(id: &ClientId, game: &Game, operation: &str) {
    let message = format!(
        "{} {} {} {} {} {} {} {} {} {} {} {} {} {}",
        operation,
        game.id,
        game.white,
        game.black,
        game.game.settings.board_size,
        game.game.settings.time_control.contingent.as_secs(),
        game.game.settings.time_control.increment.as_secs(),
        game.game.settings.half_komi,
        game.game.settings.reserve_pieces,
        game.game.settings.reserve_capstones,
        match game.game_type {
            GameType::Unrated => "1",
            GameType::Rated => "0",
            GameType::Tournament => "0",
        },
        match game.game_type {
            GameType::Unrated => "0",
            GameType::Rated => "0",
            GameType::Tournament => "1",
        },
        game.game
            .settings
            .time_control
            .extra
            .as_ref()
            .map_or("0".to_string(), |(_, extra_time)| extra_time
                .as_secs()
                .to_string()),
        game.game
            .settings
            .time_control
            .extra
            .as_ref()
            .map_or("0".to_string(), |(trigger_move, _)| trigger_move
                .to_string()),
    );
    let _ = try_send_to(id, message);
}

fn send_game_start_message(id: &ClientId, game: &Game) {
    let Some(player) = get_associated_player(&id) else {
        println!("Client {} not associated with any player", id);
        return;
    };
    let is_bot_game = fetch_player(&game.white).map_or(false, |p| p.is_bot)
        || fetch_player(&game.black).map_or(false, |p| p.is_bot);
    let message = format!(
        "Game Start {} {} vs {} {} {} {} {} {} {} {} {} {} {} {} {}",
        game.id,
        game.white,
        game.black,
        if *player == game.white {
            "white"
        } else {
            "black"
        },
        game.game.settings.board_size,
        game.game.settings.time_control.contingent.as_secs(),
        game.game.settings.time_control.increment.as_secs(),
        game.game.settings.half_komi,
        game.game.settings.reserve_pieces,
        game.game.settings.reserve_capstones,
        match game.game_type {
            GameType::Unrated => "1",
            GameType::Rated => "0",
            GameType::Tournament => "0",
        },
        match game.game_type {
            GameType::Unrated => "0",
            GameType::Rated => "0",
            GameType::Tournament => "1",
        },
        game.game
            .settings
            .time_control
            .extra
            .as_ref()
            .map_or("0".to_string(), |(trigger_move, _)| trigger_move
                .to_string()),
        game.game
            .settings
            .time_control
            .extra
            .as_ref()
            .map_or("0".to_string(), |(_, extra_time)| extra_time
                .as_secs()
                .to_string()),
        if is_bot_game { "1" } else { "0" }
    );
    let _ = try_send_to(&id, message);
}

fn send_game_action_message(id: &ClientId, game_id: &GameId, action: &TakAction) {
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
    let _ = try_send_to(&id, message);
}
