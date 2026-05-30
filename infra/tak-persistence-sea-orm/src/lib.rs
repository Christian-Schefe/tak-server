use std::time::Duration;

use async_lock::OnceCell;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use tak_core::{
    TakAsyncTimeControl, TakBaseGameSettings, TakGameSettings, TakRealtimeTimeControl, TakReserve,
    TakTimeSettings,
};
use tak_persistence_sea_orm_migrations::Migrator;
use tak_server_app::domain::RepoRetrieveError;

pub mod chat;
pub mod games;
pub mod guest;
pub mod matches;
pub mod player_account_mapping;
pub mod profile;
pub mod puzzle;
pub mod rating_history;
pub mod ratings;
pub mod stats;
pub mod tournament;

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

pub async fn create_db_pool() -> DatabaseConnection {
    DB_POOL
        .get_or_init(|| async move {
            let mariadb_database =
                std::env::var("MARIADB_DATABASE").expect("MARIADB_DATABASE must be set");
            let mariadb_user = std::env::var("MARIADB_USER").expect("MARIADB_USER must be set");
            let mariadb_password =
                std::env::var("MARIADB_PASSWORD").expect("MARIADB_PASSWORD must be set");
            let mariadb_host = std::env::var("MARIADB_HOST").expect("MARIADB_HOST must be set");
            let mariadb_port = std::env::var("MARIADB_PORT").expect("MARIADB_PORT must be set");
            let db_url = format!(
                "mysql://{}:{}@{}:{}/{}",
                mariadb_user, mariadb_password, mariadb_host, mariadb_port, mariadb_database
            );

            tracing::info!("Connecting to database at {}", db_url);

            let mut opt = ConnectOptions::new(&db_url);
            opt.max_connections(5);

            let db = try_reconnect_db_pool(opt).await;

            // Entity sync to create tables, indices, and columns
            db.get_schema_builder()
                .register(tak_persistence_sea_orm_entities::game::Entity)
                .register(tak_persistence_sea_orm_entities::player_account_mapping::Entity)
                .register(tak_persistence_sea_orm_entities::profile::Entity)
                .register(tak_persistence_sea_orm_entities::rating::Entity)
                .register(tak_persistence_sea_orm_entities::stats::Entity)
                .register(tak_persistence_sea_orm_entities::puzzle::Entity)
                .register(tak_persistence_sea_orm_entities::chat::Entity)
                .register(tak_persistence_sea_orm_entities::rating_history::Entity)
                .register(tak_persistence_sea_orm_entities::tournament_player_registration::Entity)
                .register(tak_persistence_sea_orm_entities::tournament::Entity)
                .register(tak_persistence_sea_orm_entities::matches::Entity)
                .register(tak_persistence_sea_orm_entities::tournament_round::Entity)
                .register(tak_persistence_sea_orm_entities::guest::Entity)
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

pub fn db_error_to_repo_retrieve_error(e: sea_orm::DbErr) -> RepoRetrieveError {
    match e {
        sea_orm::DbErr::RecordNotFound(_) | sea_orm::DbErr::RecordNotUpdated => {
            RepoRetrieveError::NotFound
        }
        e => RepoRetrieveError::StorageError(e.to_string()),
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonGameSettings {
    board_size: u32,
    half_komi: u32,
    pieces: u32,
    capstones: u32,
    time_settings: JsonTimeSettings,
}

impl JsonGameSettings {
    fn from_game_settings(game_settings: &TakGameSettings) -> Self {
        JsonGameSettings {
            board_size: game_settings.base.board_size,
            half_komi: game_settings.base.half_komi,
            pieces: game_settings.base.reserve.pieces,
            capstones: game_settings.base.reserve.capstones,
            time_settings: JsonTimeSettings::from_time_settings(&game_settings.time_settings),
        }
    }

    fn to_game_settings(&self) -> TakGameSettings {
        TakGameSettings {
            base: TakBaseGameSettings {
                board_size: self.board_size,
                half_komi: self.half_komi,
                reserve: TakReserve {
                    pieces: self.pieces,
                    capstones: self.capstones,
                },
            },
            time_settings: self.time_settings.to_time_settings(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum JsonTimeSettings {
    Realtime(JsonRealtimeTimeSettings),
    Async(JsonAsyncTimeSettings),
}

impl JsonTimeSettings {
    fn from_time_settings(time_settings: &TakTimeSettings) -> Self {
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
    fn to_time_settings(&self) -> TakTimeSettings {
        match self {
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
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonRealtimeTimeSettings {
    contingent_ms: u64,
    increment_ms: u64,
    extra: Option<JsonRealtimeTimeExtra>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonRealtimeTimeExtra {
    extra_time_ms: u64,
    extra_time_move: u32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonAsyncTimeSettings {
    increment_ms: u64,
}
