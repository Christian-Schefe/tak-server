use crate::game::GameSettingsResponse;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateSeekRequest {
    pub color: String,
    pub is_rated: bool,
    pub game_settings: GameSettingsResponse,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SeekResponse {
    pub id: String,
    pub creator_id: String,
    pub color: String,
    pub is_rated: bool,
    pub game_settings: GameSettingsResponse,
}
