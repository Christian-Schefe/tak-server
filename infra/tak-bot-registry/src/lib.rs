use tak_auth_ory::bot::{BotEntry, BotRepository};
use tak_server_app::domain::AccountId;

pub struct FileBotRepository {
    entries: Vec<BotEntry>,
}

#[derive(serde::Deserialize)]
pub struct JsonBotEntry {
    pub account_id: uuid::Uuid,
    pub username: String,
    pub display_name: String,
}

impl FileBotRepository {
    pub fn new() -> Self {
        let path = std::env::var("TAK_BOT_REGISTRY_PATH")
            .expect("TAK_BOT_REGISTRY_PATH environment variable not set");
        let file_content = std::fs::read_to_string(path).expect("Failed to read bot registry file");
        let entries: Vec<JsonBotEntry> =
            serde_json::from_str(&file_content).expect("Failed to parse bot registry JSON");
        Self {
            entries: entries
                .into_iter()
                .map(|e| BotEntry {
                    account_id: AccountId(e.account_id.to_string()),
                    username: e.username,
                    display_name: e.display_name,
                })
                .collect(),
        }
    }
}

impl BotRepository for FileBotRepository {
    fn get_bots(&self) -> impl Iterator<Item = &BotEntry> {
        self.entries.iter()
    }
}
