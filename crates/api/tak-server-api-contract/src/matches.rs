#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchReadinessStatusResponse {
    pub ready_player: Option<String>,
}
