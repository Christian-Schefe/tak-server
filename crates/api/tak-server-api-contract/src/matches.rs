#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchReadinessStatus {
    pub ready_player: Option<String>,
}
