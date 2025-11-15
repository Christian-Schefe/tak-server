use std::sync::{LazyLock, RwLock};

use axum::{Json, extract::State};
use tak_server_domain::{app::AppState, event::Event};

use crate::MyServiceError;

#[derive(serde::Serialize, Clone)]
pub struct JsonEventResponse {
    data: Vec<JsonEvent>,
    categories: Vec<String>,
}
#[derive(serde::Serialize, Clone)]
pub struct JsonEvent {
    name: String,
    event: String,
    category: String,
    start_date: Option<String>,
    end_date: Option<String>,
    details: Option<String>,
    registration: Option<String>,
    standings: Option<String>,
}

impl From<Event> for JsonEvent {
    fn from(event: Event) -> Self {
        Self {
            name: event.name,
            event: event.event,
            category: event.category,
            start_date: event.start_date,
            end_date: event.end_date,
            details: event.details,
            registration: event.registration,
            standings: event.standings,
        }
    }
}

pub struct EventCacheValue {
    pub response: JsonEventResponse,
    pub timestamp: std::time::Instant,
}

static EVENT_CACHE: LazyLock<RwLock<Option<EventCacheValue>>> = LazyLock::new(|| RwLock::new(None));

#[axum::debug_handler]
pub async fn get_all_events(
    State(app_state): State<AppState>,
) -> Result<Json<JsonEventResponse>, MyServiceError> {
    if let Some(cached) = EVENT_CACHE.read().unwrap().as_ref()
        && cached.timestamp.elapsed() < std::time::Duration::from_secs(300)
    {
        return Ok(Json(cached.response.clone()));
    }

    let events = app_state
        .event_service
        .list_events()
        .await
        .map_err(MyServiceError::from)?;

    let categories: Vec<String> = {
        let mut cats: Vec<String> = events
            .iter()
            .filter_map(|e| {
                let c = e.category.trim();
                if !c.is_empty() && c != "All" {
                    Some(e.category.clone())
                } else {
                    None
                }
            })
            .collect::<std::collections::HashSet<String>>()
            .into_iter()
            .collect();
        cats.sort();
        cats.insert(0, "All".to_string());
        cats
    };

    let json_events: Vec<JsonEvent> = events.into_iter().map(JsonEvent::from).collect();

    let response = JsonEventResponse {
        data: json_events,
        categories,
    };

    let mut cache_write = EVENT_CACHE.write().unwrap();
    *cache_write = Some(EventCacheValue {
        response: response.clone(),
        timestamp: std::time::Instant::now(),
    });

    Ok(Json(response))
}
