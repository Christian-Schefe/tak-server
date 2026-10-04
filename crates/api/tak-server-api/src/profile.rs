use std::str::FromStr;

use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::header,
    response::Response,
    routing::{get, post},
};
use tak_server_app::{
    domain::{AccountId, profile::ProfilePictureFileType},
    workflow::account::AccountProfileView,
};

use crate::{AppState, ServiceError, auth::Auth};

pub fn register_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/{account_id}", get(get_account_profile))
        .route("/{account_id}", post(update_account_profile))
        .route("/{account_id}/picture", get(get_profile_picture))
        .route("/{account_id}/picture", post(set_profile_picture))
}

async fn get_account_profile(
    State(app): State<AppState>,
    Path(account_id): Path<String>,
) -> Result<Json<PlayerProfileInfo>, ServiceError> {
    let Ok(account_id) = AccountId::try_from(account_id) else {
        return Err(ServiceError::BadRequest(
            "Invalid account ID format".to_string(),
        ));
    };
    let Some(_) = app.auth.get_account(&account_id).await else {
        return Err(ServiceError::NotFound("Account not found".to_string()));
    };
    let profile = app
        .app
        .get_profile_use_case
        .get_profile(&account_id)
        .await
        .map_err(|_| ServiceError::Internal("Failed to retrieve player profile".to_string()))?;

    Ok(Json(profile.into()))
}

async fn update_account_profile(
    auth: Auth,
    State(app): State<AppState>,
    Path(account_id): Path<String>,
    Json(payload): Json<PlayerProfileUpdate>,
) -> Result<(), ServiceError> {
    let Ok(account_id) = AccountId::try_from(account_id) else {
        return Err(ServiceError::BadRequest(
            "Invalid account ID format".to_string(),
        ));
    };
    if account_id != auth.account_id {
        return Err(ServiceError::Forbidden(
            "You can only update your own profile".to_string(),
        ));
    }
    let country = match payload.country {
        Some(country_str) => Some(
            country_code_enum::CountryCode::from_str(&country_str)
                .map_err(|_| ServiceError::BadRequest("Invalid country code".to_string()))?,
        ),
        None => None,
    };

    app.app
        .update_profile_use_case
        .update_profile(&auth.account_id, payload.display_name, country)
        .await
        .map_err(|_| ServiceError::Internal("Failed to update player profile".to_string()))
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
struct VersionQuery {
    #[serde(rename = "v")]
    version: Option<u64>,
}

async fn get_profile_picture(
    State(app): State<AppState>,
    Path(account_id): Path<String>,
    Query(_version_query): Query<VersionQuery>,
) -> Result<Response, ServiceError> {
    let Ok(account_id) = AccountId::try_from(account_id) else {
        return Err(ServiceError::BadRequest(
            "Invalid account ID format".to_string(),
        ));
    };
    let Some(_) = app.auth.get_account(&account_id).await else {
        return Err(ServiceError::NotFound("Account not found".to_string()));
    };
    let profile_picture = app
        .app
        .get_profile_use_case
        .get_profile_picture(&account_id)
        .await
        .map_err(|_| ServiceError::Internal("Failed to retrieve profile picture".to_string()))?;
    let Some(profile_picture) = profile_picture else {
        return Err(ServiceError::NotFound(
            "Profile picture not found".to_string(),
        ));
    };
    let body = Body::from_stream(profile_picture.stream);
    let mut response = Response::new(body);
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static(match profile_picture.content_type {
            ProfilePictureFileType::WebP => "image/webp",
        }),
    );
    Ok(response)
}

async fn set_profile_picture(
    auth: Auth,
    State(app): State<AppState>,
    Path(account_id): Path<String>,
    mut multipart: Multipart,
) -> Result<(), ServiceError> {
    let Ok(account_id) = AccountId::try_from(account_id) else {
        return Err(ServiceError::BadRequest(
            "Invalid account ID format".to_string(),
        ));
    };
    if account_id != auth.account_id {
        return Err(ServiceError::Forbidden(
            "You can only update your own profile".to_string(),
        ));
    }
    while let Some(field) = match multipart.next_field().await {
        Ok(field) => field,
        Err(e) => {
            return Err(ServiceError::BadRequest(format!(
                "Failed to parse multipart: {}",
                e
            )));
        }
    } {
        let data = field.bytes().await.map_err(|e| {
            ServiceError::BadRequest(format!("Failed to read multipart field: {}", e))
        })?;

        let img = match image::load_from_memory(&data) {
            Ok(img) => img,
            Err(e) => {
                return Err(ServiceError::BadRequest(format!(
                    "Failed to parse image data: {}",
                    e
                )));
            }
        };

        match app
            .app
            .update_profile_use_case
            .set_profile_picture(&auth.account_id, img)
            .await
        {
            Ok(_) => (),
            Err(_) => {
                return Err(ServiceError::Internal(
                    "Failed to update profile picture".to_string(),
                ));
            }
        }
    }
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct PlayerProfileUpdate {
    display_name: Option<String>,
    country: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct PlayerProfileInfo {
    display_name: Option<String>,
    country: Option<String>,
    profile_picture_version: Option<u64>,
}

impl From<AccountProfileView> for PlayerProfileInfo {
    fn from(profile: AccountProfileView) -> Self {
        PlayerProfileInfo {
            display_name: profile.display_name,
            country: profile.country.map(|c| c.to_string()),
            profile_picture_version: profile.profile_picture_version.map(|x| x.0),
        }
    }
}
