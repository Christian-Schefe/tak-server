#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JsonUserType {
    Guest,
    Bot,
    Player,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JsonUserRole {
    User,
    Moderator,
    Admin,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonUserInfo {
    pub user_id: String,
    pub user_role: JsonUserRole,
    pub user_type: JsonUserType,
    pub username: String,
    pub participation_rating: Option<f64>,
}
