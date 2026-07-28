use clap::{Parser, Subcommand};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tak_auth_ory::{AuthenticationService, jwt};
use tak_bot_registry::FileBotRepository;
use tak_persistence_sea_orm::guest::GuestRepositoryImpl;
use tak_server_app::ports::authentication::AuthenticationPort;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long)]
    env: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    SetAdmin {
        #[arg(value_name = "USERNAME")]
        username: String,
    },
    GenerateToken {
        #[arg(value_name = "USERNAME")]
        username: String,
        #[arg(short, long, default_value = "3600")]
        duration: u64,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Some(env_path) = cli.env {
        dotenvy::from_path_override(&env_path)
            .expect("Failed to load environment variables from file");
        println!("Loaded environment variables from {}", env_path.display());
    }

    let bot_repository = Arc::new(FileBotRepository::new());
    let guest_repository = Arc::new(GuestRepositoryImpl::new().await);
    let auth_service = Arc::new(AuthenticationService::new(bot_repository, guest_repository));

    match cli.command {
        Commands::SetAdmin { username } => {
            let Some(acc) = auth_service.find_by_username(&username).await else {
                println!("Account with username '{}' not found", username);
                return;
            };
            println!("Setting account {:?} to admin", acc);
            match auth_service
                .set_role(
                    &acc.account_id,
                    tak_server_app::domain::moderation::AccountRole::Admin,
                )
                .await
            {
                Ok(_) => println!("Account {} set to admin", acc.account_id.to_string()),
                Err(_) => println!(
                    "Failed to set account {} to admin",
                    acc.account_id.to_string()
                ),
            }
        }
        Commands::GenerateToken { username, duration } => {
            let Some(acc) = auth_service.find_by_username(&username).await else {
                println!("Account with username '{}' not found", username);
                return;
            };
            println!(
                "Generating token for account {:?} with duration {} seconds",
                acc, duration
            );
            let token = jwt::generate_jwt(&acc.account_id, Duration::from_secs(duration));
            println!("{}", token);
        }
    }
}
