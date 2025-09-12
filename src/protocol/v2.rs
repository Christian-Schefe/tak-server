use crate::{
    client::{ClientId, get_associated_player, try_send_to},
    game::{Game, resign_game, try_do_action},
    player::{PlayerUsername, fetch_player, try_login},
    protocol::{BoxedProtocolHandler, ProtocolHandler},
    seek::{GameType, Seek, accept_seek, add_seek},
    tak::{
        TakAction, TakDir, TakGameSettings, TakGameState, TakPlayer, TakPos, TakVariant,
        TakWinReason,
    },
};

pub struct ProtocolV2Handler(ClientId);

impl ProtocolHandler for ProtocolV2Handler {
    fn new(id: ClientId) -> Self {
        ProtocolV2Handler(id)
    }

    fn clone_box(&self) -> BoxedProtocolHandler {
        Box::new(ProtocolV2Handler(self.0))
    }

    fn get_client_id(&self) -> ClientId {
        self.0
    }

    fn handle_message(&self, msg: String) {
        let parts = msg.split_whitespace().collect::<Vec<_>>();
        if parts.is_empty() {
            println!("Received empty message");
            return;
        }
        let id = self.0;
        match parts[0] {
            "Login" => handle_login_message(id, &parts),
            "Seek" => handle_seek_message(&id, &parts),
            "Accept" => handle_accept_message(&id, &parts),
            "PING" => {
                let _ = try_send_to(&id, "OK");
            }
            s if s.starts_with("Game#") => handle_game_message(&id, &parts),
            _ => {
                println!("Unknown V2 message type: {}", parts[0]);
                let _ = try_send_to(&id, "NOK");
            }
        };
    }

    fn send_new_seek_message(&self, seek: &Seek) {
        send_seek_list_message(&self.0, seek, "new");
    }
    fn send_remove_seek_message(&self, seek: &Seek) {
        send_seek_list_message(&self.0, seek, "remove");
    }

    fn send_new_game_message(&self, game: &Game) {
        send_game_list_message(&self.0, game, "Add");
    }
    fn send_remove_game_message(&self, game: &Game) {
        send_game_list_message(&self.0, game, "Remove");
    }

    fn send_game_start_message(&self, game: &Game) {
        send_game_start_message(self.0, game);
    }
    fn send_game_action_message(&self, game: &Game, action: &TakAction) {
        send_game_action_message(self.0, game, action);
    }

    fn send_game_over_message(&self, game: &Game) {
        let message = format!(
            "Game#{} Over {}",
            game.id,
            match &game.game.game_state {
                TakGameState::Ongoing => return,
                TakGameState::Win { winner, reason } => match (winner, reason) {
                    (TakPlayer::White, TakWinReason::Flats) => "F-0".to_string(),
                    (TakPlayer::Black, TakWinReason::Flats) => "0-F".to_string(),
                    (TakPlayer::White, TakWinReason::Road) => "R-0".to_string(),
                    (TakPlayer::Black, TakWinReason::Road) => "0-R".to_string(),
                    (TakPlayer::White, TakWinReason::Default) => "1-0".to_string(),
                    (TakPlayer::Black, TakWinReason::Default) => "0-1".to_string(),
                },
                TakGameState::Draw { .. } => "1/2-1/2".to_string(),
            }
        );
        let _ = try_send_to(&self.0, message);
    }
}

fn handle_login_message(id: ClientId, parts: &[&str]) {
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
        Some(crate::tak::TimeExtra {
            trigger_move: time_extra_trigger_move,
            extra_seconds: time_extra_trigger_seconds,
        })
    } else {
        None
    };

    let game_settings = TakGameSettings {
        board_size,
        half_komi,
        reserve_pieces,
        reserve_capstones,
        time_contingent_seconds,
        time_increment_seconds,
        time_extra,
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
        seek.game_settings.time_contingent_seconds,
        seek.game_settings.time_increment_seconds,
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
            .time_extra
            .as_ref()
            .map(|x| x.trigger_move)
            .unwrap_or(0),
        seek.game_settings
            .time_extra
            .as_ref()
            .map(|x| x.extra_seconds)
            .unwrap_or(0),
        seek.opponent.as_deref().unwrap_or(""),
        fetch_player(&seek.creator).map_or("0", |p| if p.is_bot { "1" } else { "0" })
    );
    let _ = try_send_to(id, message);
}

fn send_game_list_message(id: &ClientId, game: &Game, operation: &str) {
    let message = format!(
        "GameList {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
        operation,
        game.id,
        game.white,
        game.black,
        game.game.settings.board_size,
        game.game.settings.time_contingent_seconds,
        game.game.settings.time_increment_seconds,
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
            .time_extra
            .as_ref()
            .map_or("0".to_string(), |x| x.extra_seconds.to_string()),
        game.game
            .settings
            .time_extra
            .as_ref()
            .map_or("0".to_string(), |x| x.trigger_move.to_string()),
    );
    let _ = try_send_to(id, message);
}

fn send_game_start_message(id: ClientId, game: &Game) {
    let Some(player) = get_associated_player(&id) else {
        println!("Client {} not associated with any player", id);
        return;
    };
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
        game.game.settings.time_contingent_seconds,
        game.game.settings.time_increment_seconds,
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
            .time_extra
            .as_ref()
            .map(|x| x.trigger_move)
            .unwrap_or(0),
        game.game
            .settings
            .time_extra
            .as_ref()
            .map(|x| x.extra_seconds)
            .unwrap_or(0),
        "0"
    );
    let _ = try_send_to(&id, message);
}

fn send_game_action_message(id: ClientId, game: &Game, action: &TakAction) {
    let message = match action {
        TakAction::Place { pos, variant } => format!(
            "Game#{} P {}{} {}",
            game.id,
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
                game.id,
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
