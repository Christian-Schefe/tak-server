use crate::game::GameSettingsInfo;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateSeekPayload {
    pub opponent_id: Option<String>,
    pub color: String,
    pub is_rated: bool,
    pub game_settings: GameSettingsInfo,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SeekInfo {
    pub id: String,
    pub creator_id: String,
    pub opponent_id: Option<String>,
    pub color: String,
    pub is_rated: bool,
    pub game_settings: GameSettingsInfo,
}
