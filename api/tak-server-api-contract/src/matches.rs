#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RematchStatus {
    pub rematch_requested_by: Option<String>,
}
