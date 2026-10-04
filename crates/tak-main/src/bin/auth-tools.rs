use clap::{Parser, Subcommand};
use std::{path::PathBuf, sync::Arc};
use tak_auth::AuthService;
use tak_server_app::{domain::AccountId, ports::authentication::AuthenticationPort};

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
        #[arg(value_name = "Account Id")]
        id: String,
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

    let account_repo =
        Arc::new(tak_persistence_sea_orm::account::AccountRepositoryImpl::new().await);
    let auth_service = Arc::new(AuthService::new(account_repo));

    match cli.command {
        Commands::SetAdmin { id } => {
            let account_id = AccountId::try_from(id.clone()).expect("Invalid account ID format");
            let Some(acc) = auth_service.get_account(&account_id).await else {
                println!("Account with id '{}' not found", id);
                return;
            };
            println!("Setting account {:?} to admin", acc);
            match auth_service
                .set_role(
                    &account_id,
                    tak_server_app::domain::moderation::AccountRole::Admin,
                )
                .await
            {
                Ok(_) => println!("Account {} set to admin", account_id.to_string()),
                Err(_) => println!("Failed to set account {} to admin", account_id.to_string()),
            }
        }
    }
}
