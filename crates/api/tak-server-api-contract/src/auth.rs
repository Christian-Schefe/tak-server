#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityInfo {
    pub user_id: String,
    pub user_type: JsonUserType,
    pub user_role: JsonUserRole,
    pub jwt: String,
}

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
