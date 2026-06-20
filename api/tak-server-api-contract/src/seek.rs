use crate::game::JsonGameSettings;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateSeekPayload {
    pub color: String,
    pub is_rated: bool,
    pub game_settings: JsonGameSettings,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JsonSeek {
    pub id: String,
    pub creator_id: String,
    pub color: String,
    pub is_rated: bool,
    pub game_settings: JsonGameSettings,
}
