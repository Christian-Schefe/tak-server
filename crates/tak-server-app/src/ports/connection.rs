use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use tak_domain::UserId;

pub trait UserOnlineStatusPort {
    fn set_user_online(&self, user_id: UserId) -> Option<Vec<UserId>>;
    fn set_user_offline(&self, user_id: UserId) -> Option<Vec<UserId>>;
    fn get_online_users(&self) -> Vec<UserId>;
}

pub struct UserOnlineStatusService {
    online_accounts: Arc<RwLock<HashSet<UserId>>>,
}

impl UserOnlineStatusService {
    pub fn new() -> Self {
        Self {
            online_accounts: Arc::new(RwLock::new(HashSet::new())),
        }
    }
}

impl UserOnlineStatusPort for UserOnlineStatusService {
    fn set_user_online(&self, user_id: UserId) -> Option<Vec<UserId>> {
        let mut online_accounts = self.online_accounts.write().expect("Poisoned lock");
        if online_accounts.insert(user_id.clone()) {
            Some(online_accounts.iter().cloned().collect())
        } else {
            None
        }
    }

    fn set_user_offline(&self, user_id: UserId) -> Option<Vec<UserId>> {
        let mut online_accounts = self.online_accounts.write().expect("Poisoned lock");
        if online_accounts.remove(&user_id) {
            Some(online_accounts.iter().cloned().collect())
        } else {
            None
        }
    }

    fn get_online_users(&self) -> Vec<UserId> {
        self.online_accounts
            .read()
            .expect("Poisoned lock")
            .iter()
            .cloned()
            .collect()
    }
}
