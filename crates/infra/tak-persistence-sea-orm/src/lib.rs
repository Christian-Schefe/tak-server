use std::time::{Duration, SystemTime};

use async_lock::OnceCell;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use tak_core::{
    TakActionRecord, TakAsyncTimeControl, TakBaseGameSettings, TakClock, TakGameSettings,
    TakOpening, TakPlayer, TakRealtimeTimeControl, TakReserve, TakTimeInfo, TakTimeSettings,
    ptn::{action_from_ptn, action_to_ptn, game_result_from_string, game_result_to_string},
};
use tak_domain::{
    MatchId, UserId,
    game::{GameEvent, GameEventType, GameRequests},
    matches::{MatchMode, MatchSettings, MatchStatus},
    tournament::{TournamentFormat, TournamentRound, TournamentStatus},
};
use tak_persistence_sea_orm_entities::{
    JsonActionRecord, JsonAsyncTimeSettings, JsonClock, JsonGameEvent, JsonGameEventType,
    JsonGameRequests, JsonGameSettings, JsonHistory, JsonMatchMode, JsonMatchSettings,
    JsonMatchStatus, JsonRealtimeTimeExtra, JsonRealtimeTimeSettings, JsonTakOpening,
    JsonTakPlayer, JsonTimeInfo, JsonTimeSettings, JsonTournamentFormat, JsonTournamentRound,
    JsonTournamentStatus,
};
use tak_persistence_sea_orm_migrations::Migrator;

pub mod account;
pub mod chat;
pub mod game_history;
pub mod game_state;
pub mod games;
pub mod matches;
pub mod moderation;
pub mod profile;
pub mod rating_history;
pub mod ratings;
pub mod stats;
pub mod tournament;
pub mod transaction;

static DB_POOL: OnceCell<DatabaseConnection> = OnceCell::new();

async fn try_reconnect_db_pool(opt: ConnectOptions) -> DatabaseConnection {
    loop {
        match Database::connect(opt.clone()).await {
            Ok(db) => return db,
            Err(e) => {
                tracing::error!(
                    "Failed to connect to database: {}. Retrying in 5 seconds...",
                    e
                );
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}

async fn create_db_pool() -> DatabaseConnection {
    DB_POOL
        .get_or_init(|| async move {
            let postgres_database = std::env::var("POSTGRES_DB").expect("POSTGRES_DB must be set");
            let postgres_user = std::env::var("POSTGRES_USER").expect("POSTGRES_USER must be set");
            let postgres_password =
                std::env::var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD must be set");
            let postgres_host = std::env::var("POSTGRES_HOST").expect("POSTGRES_HOST must be set");
            let postgres_port = std::env::var("POSTGRES_PORT").expect("POSTGRES_PORT must be set");
            let db_url = format!(
                "postgres://{}:{}@{}:{}/{}",
                postgres_user, postgres_password, postgres_host, postgres_port, postgres_database
            );

            tracing::info!("Connecting to database at {}", db_url);

            let mut opt = ConnectOptions::new(&db_url);
            opt.max_connections(5);

            let db = try_reconnect_db_pool(opt).await;

            // Entity sync to create tables, indices, and columns
            db.get_schema_builder()
                .register(tak_persistence_sea_orm_entities::game::Entity)
                .register(tak_persistence_sea_orm_entities::profile::Entity)
                .register(tak_persistence_sea_orm_entities::rating::Entity)
                .register(tak_persistence_sea_orm_entities::stats::Entity)
                .register(tak_persistence_sea_orm_entities::chat::Entity)
                .register(tak_persistence_sea_orm_entities::rating_history::Entity)
                .register(tak_persistence_sea_orm_entities::tournament_players::Entity)
                .register(tak_persistence_sea_orm_entities::tournament::Entity)
                .register(tak_persistence_sea_orm_entities::matches::Entity)
                .register(tak_persistence_sea_orm_entities::tournament_round::Entity)
                .register(tak_persistence_sea_orm_entities::account::Entity)
                .register(tak_persistence_sea_orm_entities::moderation::Entity)
                .register(tak_persistence_sea_orm_entities::game_state::Entity)
                .register(tak_persistence_sea_orm_entities::game_history::Entity)
                .sync(&db)
                .await
                .expect("Failed to apply entity sync");

            // Migrations to clean up / move data
            Migrator::up(&db, None)
                .await
                .expect("Failed to run migrations");

            db
        })
        .await
        .clone()
}

fn game_settings_to_json(game_settings: &TakGameSettings) -> JsonGameSettings {
    JsonGameSettings {
        board_size: game_settings.base.board_size,
        half_komi: game_settings.base.half_komi,
        pieces: game_settings.base.reserve.pieces,
        capstones: game_settings.base.reserve.capstones,
        opening: tak_opening_to_json(&game_settings.base.opening),
        time_settings: time_settings_to_json(&game_settings.time_settings),
    }
}

fn game_settings_from_json(game_settings: &JsonGameSettings) -> TakGameSettings {
    TakGameSettings {
        base: TakBaseGameSettings {
            board_size: game_settings.board_size,
            half_komi: game_settings.half_komi,
            reserve: TakReserve {
                pieces: game_settings.pieces,
                capstones: game_settings.capstones,
            },
            opening: tak_opening_from_json(&game_settings.opening),
        },
        time_settings: time_settings_from_json(&game_settings.time_settings),
    }
}

fn tak_player_to_json(player: &TakPlayer) -> JsonTakPlayer {
    match player {
        TakPlayer::White => JsonTakPlayer::White,
        TakPlayer::Black => JsonTakPlayer::Black,
    }
}

fn tak_player_from_json(player: &JsonTakPlayer) -> TakPlayer {
    match player {
        JsonTakPlayer::White => TakPlayer::White,
        JsonTakPlayer::Black => TakPlayer::Black,
    }
}

fn tak_opening_to_json(opening: &TakOpening) -> JsonTakOpening {
    match opening {
        TakOpening::Swap => JsonTakOpening::Swap,
        TakOpening::NoSwap => JsonTakOpening::NoSwap,
        TakOpening::DoubleStack => JsonTakOpening::DoubleStack,
    }
}

fn tak_opening_from_json(opening: &JsonTakOpening) -> TakOpening {
    match opening {
        JsonTakOpening::Swap => TakOpening::Swap,
        JsonTakOpening::NoSwap => TakOpening::NoSwap,
        JsonTakOpening::DoubleStack => TakOpening::DoubleStack,
    }
}

fn time_settings_to_json(time_settings: &TakTimeSettings) -> JsonTimeSettings {
    match time_settings {
        TakTimeSettings::Realtime(settings) => {
            JsonTimeSettings::Realtime(JsonRealtimeTimeSettings {
                contingent_ms: settings.contingent.as_millis() as u64,
                increment_ms: settings.increment.as_millis() as u64,
                extra: settings.extra.as_ref().map(|(trigger_move, extra_time)| {
                    JsonRealtimeTimeExtra {
                        extra_time_ms: extra_time.as_millis() as u64,
                        extra_time_move: *trigger_move,
                    }
                }),
            })
        }

        TakTimeSettings::Async(settings) => JsonTimeSettings::Async(JsonAsyncTimeSettings {
            increment_ms: settings.contingent.as_millis() as u64,
        }),
    }
}

fn time_settings_from_json(time_settings: &JsonTimeSettings) -> TakTimeSettings {
    match time_settings {
        JsonTimeSettings::Realtime(json_settings) => {
            TakTimeSettings::Realtime(TakRealtimeTimeControl {
                contingent: Duration::from_millis(json_settings.contingent_ms),
                increment: Duration::from_millis(json_settings.increment_ms),
                extra: if let Some(extra) = &json_settings.extra {
                    Some((
                        extra.extra_time_move,
                        Duration::from_millis(extra.extra_time_ms),
                    ))
                } else {
                    None
                },
            })
        }
        JsonTimeSettings::Async(json_settings) => TakTimeSettings::Async(TakAsyncTimeControl {
            contingent: Duration::from_millis(json_settings.increment_ms as u64),
        }),
    }
}

fn game_event_to_json(event: &GameEvent) -> JsonGameEvent {
    JsonGameEvent {
        remaining_ms: time_info_to_json(&event.time_info),
        event_type: match &event.event_type {
            GameEventType::ActionPerformed {
                action,
                player,
                ply_index,
            } => JsonGameEventType::Action {
                action: action_to_ptn(&action),
                player: tak_player_to_json(&player),
                ply_index: *ply_index as u32,
            },
            GameEventType::ActionUndone { ply_index } => JsonGameEventType::Undo {
                ply_index: *ply_index as u32,
            },
            GameEventType::GameOver { result } => JsonGameEventType::GameOver {
                result: game_result_to_string(*result),
            },
        },
    }
}

fn game_event_from_json(event: &JsonGameEvent) -> Result<GameEvent, String> {
    Ok(GameEvent {
        time_info: time_info_from_json(&event.remaining_ms),
        event_type: match &event.event_type {
            JsonGameEventType::Action {
                action,
                player,
                ply_index,
            } => GameEventType::ActionPerformed {
                player: tak_player_from_json(&player),
                ply_index: *ply_index as usize,
                action: action_from_ptn(&action)
                    .ok_or_else(|| format!("Failed to parse action from PTN"))?,
            },
            JsonGameEventType::Undo { ply_index } => GameEventType::ActionUndone {
                ply_index: *ply_index as usize,
            },
            JsonGameEventType::GameOver { result } => GameEventType::GameOver {
                result: game_result_from_string(&result)
                    .ok_or_else(|| format!("Failed to parse game result from string"))?,
            },
        },
    })
}

fn time_info_to_json(time_info: &TakTimeInfo) -> JsonTimeInfo {
    JsonTimeInfo {
        white_remaining_ms: time_info.white_remaining.as_millis() as u64,
        black_remaining_ms: time_info.black_remaining.as_millis() as u64,
    }
}

fn time_info_from_json(time_info: &JsonTimeInfo) -> TakTimeInfo {
    TakTimeInfo {
        white_remaining: Duration::from_millis(time_info.white_remaining_ms),
        black_remaining: Duration::from_millis(time_info.black_remaining_ms),
    }
}

fn history_to_json(history: &Vec<TakActionRecord>) -> JsonHistory {
    JsonHistory(
        history
            .iter()
            .map(|action_record| JsonActionRecord {
                action: action_to_ptn(&action_record.action),
                time_info: time_info_to_json(&action_record.time_info),
            })
            .collect(),
    )
}

fn history_from_json(history: &JsonHistory) -> Result<Vec<TakActionRecord>, String> {
    history
        .0
        .iter()
        .map(|json_action_record| {
            Ok(TakActionRecord {
                action: action_from_ptn(&json_action_record.action)
                    .ok_or_else(|| format!("Failed to parse action from PTN"))?,
                time_info: time_info_from_json(&json_action_record.time_info),
            })
        })
        .collect::<Result<Vec<_>, String>>()
}

fn game_requests_to_json(requests: &GameRequests) -> JsonGameRequests {
    JsonGameRequests {
        draw_offered: requests.draw_offered,
        undo_requested: requests.undo_requested,
        more_time_offered: requests.more_time_offered.map(|d| d.as_millis() as u64),
    }
}

fn game_requests_from_json(requests: &JsonGameRequests) -> GameRequests {
    GameRequests {
        draw_offered: requests.draw_offered,
        undo_requested: requests.undo_requested,
        more_time_offered: requests
            .more_time_offered
            .map(|ms| Duration::from_millis(ms)),
    }
}

fn tak_clock_to_json(clock: &TakClock<SystemTime>) -> JsonClock {
    JsonClock {
        remaining_time: time_info_to_json(&clock.remaining_time),
        is_ticking: clock.is_ticking.map(|t| t.into()),
    }
}

fn tak_clock_from_json(clock: &JsonClock) -> TakClock<SystemTime> {
    TakClock {
        remaining_time: time_info_from_json(&clock.remaining_time),
        is_ticking: clock.is_ticking.map(|t| t.into()),
    }
}

fn match_settings_to_json(settings: &MatchSettings) -> JsonMatchSettings {
    JsonMatchSettings {
        game_settings: game_settings_to_json(&settings.game_settings),
        match_mode: match_mode_to_json(&settings.match_mode),
        is_rated: settings.is_rated,
    }
}

fn match_settings_from_json(settings: &JsonMatchSettings) -> MatchSettings {
    MatchSettings {
        game_settings: game_settings_from_json(&settings.game_settings),
        match_mode: match_mode_from_json(&settings.match_mode),
        is_rated: settings.is_rated,
    }
}

fn match_mode_to_json(match_mode: &MatchMode) -> JsonMatchMode {
    match match_mode {
        MatchMode::Unlimited => JsonMatchMode::Unlimited,
        MatchMode::FixedGames(games) => JsonMatchMode::FixedGames(*games),
        MatchMode::FirstTo(games) => JsonMatchMode::FirstTo(*games),
    }
}

fn match_mode_from_json(match_mode: &JsonMatchMode) -> MatchMode {
    match match_mode {
        JsonMatchMode::Unlimited => MatchMode::Unlimited,
        JsonMatchMode::FixedGames(games) => MatchMode::FixedGames(*games),
        JsonMatchMode::FirstTo(games) => MatchMode::FirstTo(*games),
    }
}

fn match_status_to_json(status: &MatchStatus) -> JsonMatchStatus {
    match status {
        MatchStatus::Waiting => JsonMatchStatus::Waiting,
        MatchStatus::Ongoing => JsonMatchStatus::Ongoing,
        MatchStatus::Completed => JsonMatchStatus::Completed,
    }
}

fn match_status_from_json(status: &JsonMatchStatus) -> MatchStatus {
    match status {
        JsonMatchStatus::Waiting => MatchStatus::Waiting,
        JsonMatchStatus::Ongoing => MatchStatus::Ongoing,
        JsonMatchStatus::Completed => MatchStatus::Completed,
    }
}

fn tournament_format_to_json(format: &TournamentFormat) -> JsonTournamentFormat {
    match format {
        TournamentFormat::Swiss { rounds } => JsonTournamentFormat::Swiss {
            rounds: *rounds as u32,
        },
        TournamentFormat::GroupRoundRobin { group_size } => JsonTournamentFormat::GroupRoundRobin {
            group_size: *group_size as u32,
        },
        TournamentFormat::RoundRobin => JsonTournamentFormat::RoundRobin,
    }
}

fn tournament_format_from_json(format: &JsonTournamentFormat) -> TournamentFormat {
    match format {
        JsonTournamentFormat::Swiss { rounds } => TournamentFormat::Swiss {
            rounds: *rounds as usize,
        },
        JsonTournamentFormat::RoundRobin => TournamentFormat::RoundRobin,
        JsonTournamentFormat::GroupRoundRobin { group_size } => TournamentFormat::GroupRoundRobin {
            group_size: *group_size as usize,
        },
    }
}

fn tournament_status_to_json(status: &TournamentStatus) -> JsonTournamentStatus {
    match status {
        TournamentStatus::Upcoming { registration_open } => JsonTournamentStatus::Upcoming {
            registration_open: *registration_open,
        },
        TournamentStatus::Ongoing => JsonTournamentStatus::Ongoing,
        TournamentStatus::Completed => JsonTournamentStatus::Completed,
    }
}

fn tournament_status_from_json(status: &JsonTournamentStatus) -> TournamentStatus {
    match status {
        JsonTournamentStatus::Upcoming { registration_open } => TournamentStatus::Upcoming {
            registration_open: *registration_open,
        },
        JsonTournamentStatus::Ongoing => TournamentStatus::Ongoing,
        JsonTournamentStatus::Completed => TournamentStatus::Completed,
    }
}

fn tournament_round_to_json(round: &TournamentRound) -> JsonTournamentRound {
    JsonTournamentRound {
        byes: round.byes.iter().map(|id| id.0).collect(),
        matches: round.matches.iter().map(|id| id.0).collect(),
    }
}

fn tournament_round_from_json(round: &JsonTournamentRound) -> TournamentRound {
    TournamentRound {
        byes: round.byes.iter().map(|id| UserId(*id)).collect(),
        matches: round.matches.iter().map(|id| MatchId(*id)).collect(),
    }
}
