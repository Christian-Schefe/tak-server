use std::sync::Arc;

use ory_kratos_client::{
    apis::{
        configuration::Configuration,
        frontend_api::{to_session, update_login_flow},
        identity_api::{create_identity, get_identity, list_identities, patch_identity},
    },
    models::{
        self, SuccessfulNativeLogin, UpdateLoginFlowWithPasswordMethod,
        UpdateSettingsFlowWithPasswordMethod,
    },
};
use tak_server_app::{
    domain::{
        AccountId,
        moderation::{AccountRole, ModerationFlag, ModerationFlags},
    },
    ports::authentication::{Account, AccountType},
};

mod api;

pub struct OryAuthenticationService {
    public_config: Arc<Configuration>,
    admin_config: Arc<Configuration>,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
enum OryAccountRole {
    #[default]
    User,
    Moderator,
    Admin,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
enum OryAccountType {
    #[default]
    Player,
    Bot,
}

#[derive(serde::Deserialize, Debug)]
struct OryTraits {
    pub email: Option<String>,
    pub username: String,
    pub display_name: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
struct OryAdminMetadata {
    #[serde(default)]
    role: OryAccountRole,
    #[serde(default)]
    banned: bool,
    #[serde(default)]
    silenced: bool,
    #[serde(default)]
    account_type: OryAccountType,
}

async fn do_login_flow(
    config: &Configuration,
    identifier: &str,
    password: &str,
) -> Result<SuccessfulNativeLogin, String> {
    let flow_id =
        api::create_native_login_flow(config, None, None, None, None, None, None, None, None)
            .await
            .map_err(|e| e.to_string())?;

    let res = update_login_flow(
        config,
        &flow_id,
        models::UpdateLoginFlowBody::Password(Box::new(UpdateLoginFlowWithPasswordMethod {
            csrf_token: None,
            identifier: identifier.to_string(),
            method: "password".to_string(),
            password: password.to_string(),
            password_identifier: None,
            transient_payload: None,
        })),
        None,
        None,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(res)
}

impl OryAuthenticationService {
    pub fn new() -> Self {
        let kratos_public_base_url = std::env::var("TAK_ORY_KRATOS_PUBLIC_URL")
            .expect("TAK_ORY_KRATOS_PUBLIC_URL env var not set");
        let kratos_admin_base_url = std::env::var("TAK_ORY_KRATOS_ADMIN_URL")
            .expect("TAK_ORY_KRATOS_ADMIN_URL env var not set");

        let client = reqwest::Client::new();
        Self {
            admin_config: Arc::new(Configuration {
                base_path: kratos_admin_base_url,
                client: client.clone(),
                ..Default::default()
            }),
            public_config: Arc::new(Configuration {
                base_path: kratos_public_base_url,
                client,
                ..Default::default()
            }),
        }
    }

    pub async fn get_account_by_cookie(&self, cookie: &str) -> Option<Account> {
        to_session(&self.public_config, None, Some(cookie), None)
            .await
            .ok()
            .and_then(|session| {
                let identity = session.identity?;
                Self::identity_to_account(*identity)
            })
    }

    pub async fn create_account(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
    ) -> Result<Account, String> {
        let identity = models::CreateIdentityBody {
            credentials: Some(Box::new(models::IdentityWithCredentials {
                oidc: None,
                password: Some(Box::new(models::IdentityWithCredentialsPassword {
                    config: Some(Box::new(models::IdentityWithCredentialsPasswordConfig {
                        hashed_password: Some(password_hash.to_string()),
                        password: None,
                        use_password_migration_hook: None,
                    })),
                })),
                saml: None,
            })),
            external_id: None,
            metadata_admin: Some(Some(
                serde_json::to_value(OryAdminMetadata {
                    role: OryAccountRole::User,
                    banned: false,
                    silenced: false,
                    account_type: OryAccountType::Player,
                })
                .unwrap(),
            )),
            metadata_public: None,
            organization_id: None,
            recovery_addresses: None,
            schema_id: "default".to_string(),
            state: None,
            traits: serde_json::json!({
                "username": username,
                "email": email,
                "display_name": username,
            }),
            verifiable_addresses: None,
        };

        match create_identity(self.admin_config.as_ref(), Some(identity)).await {
            Ok(response) => {
                let account = Self::identity_to_account(response)
                    .ok_or("Failed to convert identity".to_string())?;
                Ok(account)
            }
            Err(error) => {
                log::error!("Failed to create identity: {:?}", error);
                Err(error.to_string())
            }
        }
    }

    pub async fn login_username_password(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Account, String> {
        let res = do_login_flow(&self.public_config, username, password).await?;
        let identity = res
            .session
            .identity
            .ok_or("No identity in session".to_string())?;
        Self::identity_to_account(*identity).ok_or("Failed to convert identity".to_string())
    }

    pub async fn change_password(
        &self,
        username: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), String> {
        let login_res = do_login_flow(&self.public_config, username, old_password).await?;

        let flow_id = api::create_native_settings_flow(
            &self.public_config,
            login_res.session_token.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?;
        api::update_settings_flow(
            &self.public_config,
            &flow_id,
            models::UpdateSettingsFlowBody::Password(Box::new(
                UpdateSettingsFlowWithPasswordMethod {
                    csrf_token: None,
                    method: "password".to_string(),
                    password: new_password.to_string(),
                    transient_payload: None,
                },
            )),
            login_res.session_token.as_deref(),
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn find_by_username(&self, username: &str) -> Option<Account> {
        let identities = list_identities(
            &self.admin_config,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(username),
            None,
            None,
            None,
        )
        .await
        .ok()?;
        // TODO: Handle multiple identities with the same identifier in different credential categories
        let first_identity = identities.into_iter().next()?;
        Self::identity_to_account(first_identity)
    }

    fn identity_to_account(identity: models::Identity) -> Option<Account> {
        let metadata: OryAdminMetadata = identity.metadata_admin.flatten().map_or_else(
            || OryAdminMetadata::default(),
            |x| serde_json::from_value(x).unwrap_or_default(),
        );

        let traits: OryTraits = identity
            .traits
            .map(|x| serde_json::from_value(x).ok())
            .flatten()?;

        let account_type = match metadata.account_type {
            OryAccountType::Player => AccountType::Player,
            OryAccountType::Bot => AccountType::Bot,
        };
        let role = match metadata.role {
            OryAccountRole::User => AccountRole::User,
            OryAccountRole::Moderator => AccountRole::Moderator,
            OryAccountRole::Admin => AccountRole::Admin,
        };
        let flags = {
            let mut moderation_flags = ModerationFlags::new();
            if metadata.banned {
                moderation_flags.set_flag(ModerationFlag::Banned);
            }
            if metadata.silenced {
                moderation_flags.set_flag(ModerationFlag::Silenced);
            }
            moderation_flags
        };

        let account = Account::new(
            AccountId(identity.id),
            account_type,
            role,
            flags,
            traits.username,
            traits.display_name,
            traits.email,
        );
        Some(account)
    }

    pub async fn get_account(&self, account_id: &AccountId) -> Option<Account> {
        let identity = match get_identity(&self.admin_config, &account_id.to_string(), None).await {
            Ok(response) => response,
            Err(_) => return None,
        };

        let account = Self::identity_to_account(identity)?;
        Some(account)
    }

    pub async fn set_role(&self, account_id: &AccountId, role: AccountRole) -> Result<(), ()> {
        let ory_role = match role {
            AccountRole::User => OryAccountRole::User,
            AccountRole::Moderator => OryAccountRole::Moderator,
            AccountRole::Admin => OryAccountRole::Admin,
        };
        let json_patch = vec![models::JsonPatch {
            op: "add".to_string(),
            path: "/metadata_admin/role".to_string(),
            value: Some(Some(serde_json::to_value(ory_role).map_err(|_| ())?)),
            from: None,
        }];
        match patch_identity(self.admin_config.as_ref(), &account_id.0, Some(json_patch)).await {
            Ok(_) => {}
            Err(_) => return Err(()),
        };
        Ok(())
    }

    pub async fn add_flag(&self, account_id: &AccountId, flag: ModerationFlag) -> Result<(), ()> {
        let json_patch = vec![models::JsonPatch {
            op: "add".to_string(),
            path: format!(
                "/metadata_admin/{}",
                match flag {
                    ModerationFlag::Banned => "banned",
                    ModerationFlag::Silenced => "silenced",
                }
            ),
            value: Some(Some(serde_json::json!(true))),
            from: None,
        }];
        match patch_identity(self.admin_config.as_ref(), &account_id.0, Some(json_patch)).await {
            Ok(_) => {}
            Err(_) => return Err(()),
        };
        Ok(())
    }

    pub async fn remove_flag(
        &self,
        account_id: &AccountId,
        flag: ModerationFlag,
    ) -> Result<(), ()> {
        let json_patch = vec![models::JsonPatch {
            op: "add".to_string(),
            path: format!(
                "/metadata_admin/{}",
                match flag {
                    ModerationFlag::Banned => "banned",
                    ModerationFlag::Silenced => "silenced",
                }
            ),
            value: Some(Some(serde_json::json!(false))),
            from: None,
        }];
        match patch_identity(self.admin_config.as_ref(), &account_id.0, Some(json_patch)).await {
            Ok(_) => {}
            Err(_) => return Err(()),
        };
        Ok(())
    }
}
