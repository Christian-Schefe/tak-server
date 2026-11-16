#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UserTypeResponse {
    Guest,
    Bot,
    Player,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UserRoleResponse {
    User,
    Moderator,
    Admin,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfoResponse {
    pub user_id: String,
    pub user_role: UserRoleResponse,
    pub user_type: UserTypeResponse,
    pub username: String,
    pub participation_rating: Option<f64>,
}
