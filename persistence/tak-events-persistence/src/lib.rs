use google_sheets4::{
    Sheets,
    api::Scope,
    hyper::{Client, client::HttpConnector},
    hyper_rustls::{HttpsConnector, HttpsConnectorBuilder},
    oauth2::{ServiceAccountAuthenticator, read_service_account_key},
};
use tak_server_domain::{
    ServiceError, ServiceResult,
    event::{Event, EventRepository},
};

pub struct GoogleSheetsEventRepository {
    hub: Sheets<HttpsConnector<HttpConnector>>,
    spreadsheet_id: String,
}

impl GoogleSheetsEventRepository {
    pub async fn new() -> Self {
        let key_path = std::env::var("GOOGLE_SERVICE_ACCOUNT_KEY")
            .expect("GOOGLE_SERVICE_ACCOUNT_KEY must be set");

        let spreadsheet_id = std::env::var("GOOGLE_SHEETS_EVENTS_SPREADSHEET_ID")
            .expect("GOOGLE_SHEETS_EVENTS_SPREADSHEET_ID must be set");

        let key = read_service_account_key(key_path)
            .await
            .expect("Failed to read service account key");

        let auth = ServiceAccountAuthenticator::builder(key)
            .build()
            .await
            .expect("Failed to build authenticator");

        let hub = Sheets::new(
            Client::builder().build(
                HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .expect("Failed to create native roots")
                    .https_or_http()
                    .enable_http1()
                    .enable_http2()
                    .build(),
            ),
            auth,
        );

        Self {
            hub,
            spreadsheet_id,
        }
    }
}

fn empty_to_opt(s: &str) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn validate_link(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        None
    } else if s.starts_with("http://") || s.starts_with("https://") {
        Some(s.to_string())
    } else {
        None
    }
}

#[async_trait::async_trait]
impl EventRepository for GoogleSheetsEventRepository {
    async fn get_all_events(&self) -> ServiceResult<Vec<Event>> {
        let resp = self
            .hub
            .spreadsheets()
            .values_get(&self.spreadsheet_id, "Event List!A2:H")
            .add_scope(Scope::SpreadsheetReadonly)
            .doit()
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        let rows = resp.1.values.unwrap_or_default();

        let events = rows
            .into_iter()
            .map(|row| {
                let mut row = row
                    .into_iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>();
                row.resize(8, "".to_string());

                Event {
                    name: row[0].clone(),
                    event: row[1].clone(),
                    category: row[2].clone(),
                    start_date: empty_to_opt(&row[3]),
                    end_date: empty_to_opt(&row[4]),
                    details: validate_link(&row[5]),
                    registration: validate_link(&row[6]),
                    standings: validate_link(&row[7]),
                }
            })
            .collect();

        Ok(events)
    }
}

pub struct NoopEventRepository;

#[async_trait::async_trait]
impl EventRepository for NoopEventRepository {
    async fn get_all_events(&self) -> ServiceResult<Vec<Event>> {
        Ok(Vec::new())
    }
}
