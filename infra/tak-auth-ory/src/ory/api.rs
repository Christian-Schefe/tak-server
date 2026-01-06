use ory_kratos_client::apis::Error;
use ory_kratos_client::apis::frontend_api::UpdateSettingsFlowError;
use ory_kratos_client::apis::{
    ResponseContent,
    configuration::Configuration,
    frontend_api::{CreateNativeLoginFlowError, CreateNativeSettingsFlowError},
};
use ory_kratos_client::models::UpdateSettingsFlowBody;
use serde::de::Error as _;

enum ContentType {
    Json,
    Text,
    Unsupported(String),
}

impl From<&str> for ContentType {
    fn from(content_type: &str) -> Self {
        if content_type.starts_with("application") && content_type.contains("json") {
            return Self::Json;
        } else if content_type.starts_with("text/plain") {
            return Self::Text;
        } else {
            return Self::Unsupported(content_type.to_string());
        }
    }
}

pub async fn create_native_settings_flow(
    configuration: &Configuration,
    x_session_token: Option<&str>,
) -> Result<String, ory_kratos_client::apis::Error<CreateNativeSettingsFlowError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_x_session_token = x_session_token;

    let uri_str = format!("{}/self-service/settings/api", configuration.base_path);
    let mut req_builder = configuration.client.request(reqwest::Method::GET, &uri_str);

    if let Some(ref user_agent) = configuration.user_agent {
        req_builder = req_builder.header(reqwest::header::USER_AGENT, user_agent.clone());
    }
    if let Some(param_value) = p_x_session_token {
        req_builder = req_builder.header("X-Session-Token", param_value.to_string());
    }

    let req = req_builder.build()?;
    let resp = configuration.client.execute(req).await?;

    let status = resp.status();
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream");
    let content_type = ContentType::from(content_type);

    if !status.is_client_error() && !status.is_server_error() {
        let content = resp.text().await?;
        match content_type {
            ContentType::Json => {
                let value =
                    serde_json::from_str::<serde_json::Value>(&content).map_err(Error::from)?;
                let id = value.get("id").and_then(|v| v.as_str()).ok_or(Error::from(
                    serde_json::Error::custom("Missing id field in SettingsFlow response"),
                ))?;
                Ok(id.to_string())
            }
            ContentType::Text => {
                return Err(Error::from(serde_json::Error::custom(
                    "Received `text/plain` content type response that cannot be converted to `models::SettingsFlow`",
                )));
            }
            ContentType::Unsupported(unknown_type) => {
                return Err(Error::from(serde_json::Error::custom(format!(
                    "Received `{unknown_type}` content type response that cannot be converted to `models::SettingsFlow`"
                ))));
            }
        }
    } else {
        let content = resp.text().await?;
        let entity: Option<CreateNativeSettingsFlowError> = serde_json::from_str(&content).ok();
        Err(Error::ResponseError(ResponseContent {
            status,
            content,
            entity,
        }))
    }
}

pub async fn create_native_login_flow(
    configuration: &Configuration,
    refresh: Option<bool>,
    aal: Option<&str>,
    x_session_token: Option<&str>,
    return_session_token_exchange_code: Option<bool>,
    return_to: Option<&str>,
    organization: Option<&str>,
    via: Option<&str>,
    identity_schema: Option<&str>,
) -> Result<String, Error<CreateNativeLoginFlowError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_refresh = refresh;
    let p_aal = aal;
    let p_x_session_token = x_session_token;
    let p_return_session_token_exchange_code = return_session_token_exchange_code;
    let p_return_to = return_to;
    let p_organization = organization;
    let p_via = via;
    let p_identity_schema = identity_schema;

    let uri_str = format!("{}/self-service/login/api", configuration.base_path);
    let mut req_builder = configuration.client.request(reqwest::Method::GET, &uri_str);

    if let Some(ref param_value) = p_refresh {
        req_builder = req_builder.query(&[("refresh", &param_value.to_string())]);
    }
    if let Some(ref param_value) = p_aal {
        req_builder = req_builder.query(&[("aal", &param_value.to_string())]);
    }
    if let Some(ref param_value) = p_return_session_token_exchange_code {
        req_builder = req_builder.query(&[(
            "return_session_token_exchange_code",
            &param_value.to_string(),
        )]);
    }
    if let Some(ref param_value) = p_return_to {
        req_builder = req_builder.query(&[("return_to", &param_value.to_string())]);
    }
    if let Some(ref param_value) = p_organization {
        req_builder = req_builder.query(&[("organization", &param_value.to_string())]);
    }
    if let Some(ref param_value) = p_via {
        req_builder = req_builder.query(&[("via", &param_value.to_string())]);
    }
    if let Some(ref param_value) = p_identity_schema {
        req_builder = req_builder.query(&[("identity_schema", &param_value.to_string())]);
    }
    if let Some(ref user_agent) = configuration.user_agent {
        req_builder = req_builder.header(reqwest::header::USER_AGENT, user_agent.clone());
    }
    if let Some(param_value) = p_x_session_token {
        req_builder = req_builder.header("X-Session-Token", param_value.to_string());
    }

    let req = req_builder.build()?;
    let resp = configuration.client.execute(req).await?;

    let status = resp.status();
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream");
    let content_type = ContentType::from(content_type);

    if !status.is_client_error() && !status.is_server_error() {
        let content = resp.text().await?;
        match content_type {
            ContentType::Json => {
                let value =
                    serde_json::from_str::<serde_json::Value>(&content).map_err(Error::from)?;
                let id = value.get("id").and_then(|v| v.as_str()).ok_or(Error::from(
                    serde_json::Error::custom("Missing id field in LoginFlow response"),
                ))?;
                Ok(id.to_string())
            }
            ContentType::Text => {
                return Err(Error::from(serde_json::Error::custom(
                    "Received `text/plain` content type response that cannot be converted to `models::LoginFlow`",
                )));
            }
            ContentType::Unsupported(unknown_type) => {
                return Err(Error::from(serde_json::Error::custom(format!(
                    "Received `{unknown_type}` content type response that cannot be converted to `models::LoginFlow`"
                ))));
            }
        }
    } else {
        let content = resp.text().await?;
        let entity: Option<CreateNativeLoginFlowError> = serde_json::from_str(&content).ok();
        Err(Error::ResponseError(ResponseContent {
            status,
            content,
            entity,
        }))
    }
}

pub async fn update_settings_flow(
    configuration: &Configuration,
    flow: &str,
    update_settings_flow_body: UpdateSettingsFlowBody,
    x_session_token: Option<&str>,
    cookie: Option<&str>,
) -> Result<(), Error<UpdateSettingsFlowError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_flow = flow;
    let p_update_settings_flow_body = update_settings_flow_body;
    let p_x_session_token = x_session_token;
    let p_cookie = cookie;

    let uri_str = format!("{}/self-service/settings", configuration.base_path);
    let mut req_builder = configuration
        .client
        .request(reqwest::Method::POST, &uri_str);

    req_builder = req_builder.query(&[("flow", &p_flow.to_string())]);
    if let Some(ref user_agent) = configuration.user_agent {
        req_builder = req_builder.header(reqwest::header::USER_AGENT, user_agent.clone());
    }
    if let Some(param_value) = p_x_session_token {
        req_builder = req_builder.header("X-Session-Token", param_value.to_string());
    }
    if let Some(param_value) = p_cookie {
        req_builder = req_builder.header("Cookie", param_value.to_string());
    }
    req_builder = req_builder.json(&p_update_settings_flow_body);

    let req = req_builder.build()?;
    let resp = configuration.client.execute(req).await?;

    let status = resp.status();
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream");
    let content_type = ContentType::from(content_type);

    if !status.is_client_error() && !status.is_server_error() {
        let _content = resp.text().await?;
        match content_type {
            ContentType::Json => Ok(()),
            ContentType::Text => {
                return Err(Error::from(serde_json::Error::custom(
                    "Received `text/plain` content type response that cannot be converted to `models::SettingsFlow`",
                )));
            }
            ContentType::Unsupported(unknown_type) => {
                return Err(Error::from(serde_json::Error::custom(format!(
                    "Received `{unknown_type}` content type response that cannot be converted to `models::SettingsFlow`"
                ))));
            }
        }
    } else {
        let content = resp.text().await?;
        let entity: Option<UpdateSettingsFlowError> = serde_json::from_str(&content).ok();
        Err(Error::ResponseError(ResponseContent {
            status,
            content,
            entity,
        }))
    }
}
