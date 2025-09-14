use std::time::Instant;

use crate::{
    client::{ClientId, get_associated_player, send_to},
    game::{Game, observe_game, unobserve_game},
    player::fetch_player,
    protocol::ServerMessage,
    seek::GameType,
    tak::TakPlayer,
};

pub fn handle_server_game_list_message(id: &ClientId, msg: &ServerMessage) {
    match msg {
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
        ServerMessage::GameStart { game } => {
            send_game_start_message(id, game);
        }
        ServerMessage::ObserveGame { game } => {
            send_game_string_message(id, game, "Observe");
            for action in game.game.move_history.iter() {
                super::game::send_game_action_message(id, &game.id, action);
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
            send_to(id, time_message);
        }
        _ => {
            eprintln!("Unhandled server game list message: {:?}", msg);
        }
    }
}

pub fn handle_observe_message(id: &ClientId, parts: &[&str], observe: bool) {
    if parts.len() != 2 {
        println!("Invalid Observe message format: {:?}", parts);
        send_to(id, "NOK");
        return;
    }
    let Ok(game_id) = parts[1].parse::<u32>() else {
        println!("Invalid Game ID in Observe message: {}", parts[1]);
        send_to(id, "NOK");
        return;
    };
    let Some(_) = get_associated_player(id) else {
        println!("Client {} not associated with any player", id);
        send_to(id, "NOK");
        return;
    };
    if observe {
        if let Err(e) = observe_game(id, &game_id) {
            println!("Error observing game {}: {}", game_id, e);
            send_to(id, "NOK");
            return;
        };
    } else {
        if let Err(e) = unobserve_game(id, &game_id) {
            println!("Error unobserving game {}: {}", game_id, e);
            send_to(id, "NOK");
            return;
        };
    }
}

pub fn send_game_string_message(id: &ClientId, game: &Game, operation: &str) {
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
    send_to(id, message);
}

pub fn send_game_start_message(id: &ClientId, game: &Game) {
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
    send_to(&id, message);
}
