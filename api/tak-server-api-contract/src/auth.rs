#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityInfo {
    pub account_id: String,
    pub player_id: String,
    pub is_guest: bool,
    pub new_guest: bool,
    pub jwt: String,
}
