use crate::domain::{AccountId, account::AccountRole};

pub trait AuthenticationService {
    fn get_subject(&self, subject_id: AccountId) -> Option<AuthContext>;
}

pub struct AuthContext {
    pub account_id: AccountId,
    pub subject_type: AuthSubject,
    pub role: AccountRole,
}

pub enum AuthSubject {
    Player {
        username: String,
        email: Option<String>,
    },
    Bot {
        username: String,
    },
    Guest {
        guest_number: u64,
    },
}
