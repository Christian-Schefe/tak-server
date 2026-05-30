use std::sync::Arc;

use ory_kratos_client::{
    apis::{
        configuration::Configuration,
        frontend_api::to_session,
        identity_api::{get_identity, list_identities, patch_identity},
    },
    models,
};
use tak_server_app::{
    domain::{
        AccountId,
        moderation::{AccountRole, ModerationFlag, ModerationFlags},
    },
    ports::authentication::{Account, AccountType},
};

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
            let mut moderation_flags = ModerationFlags::empty();
            if metadata.banned {
                moderation_flags.set_flag(ModerationFlag::Banned);
            }
            if metadata.silenced {
                moderation_flags.set_flag(ModerationFlag::Silenced);
            }
            moderation_flags
        };

        let account_id = AccountId::try_from(identity.id.clone()).ok()?;

        let account = Account::new(
            account_id,
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
        let id = account_id.to_string();
        let identity = match get_identity(&self.admin_config, &id, None).await {
            Ok(response) => response,
            Err(_) => return None,
        };

        let account = Self::identity_to_account(identity)?;
        Some(account)
    }

    pub async fn set_role(&self, account_id: &AccountId, role: AccountRole) -> Result<(), ()> {
        let id = account_id.to_string();
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
        match patch_identity(self.admin_config.as_ref(), &id, Some(json_patch)).await {
            Ok(_) => {}
            Err(_) => return Err(()),
        };
        Ok(())
    }

    pub async fn add_flag(&self, account_id: &AccountId, flag: ModerationFlag) -> Result<(), ()> {
        let id = account_id.to_string();
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
        match patch_identity(self.admin_config.as_ref(), &id, Some(json_patch)).await {
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
        let id = account_id.to_string();
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
        match patch_identity(self.admin_config.as_ref(), &id, Some(json_patch)).await {
            Ok(_) => {}
            Err(_) => return Err(()),
        };
        Ok(())
    }
}
