#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountRole {
    User,
    Moderator,
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModerationFlag {
    Banned,
    Silenced,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModerationFlags {
    banned: bool,
    silenced: bool,
}

impl ModerationFlags {
    pub fn new() -> Self {
        Self {
            banned: false,
            silenced: false,
        }
    }

    pub fn set_flag(&mut self, flag: ModerationFlag) {
        match flag {
            ModerationFlag::Banned => self.banned = true,
            ModerationFlag::Silenced => self.silenced = true,
        }
    }

    pub fn unset_flag(&mut self, flag: ModerationFlag) {
        match flag {
            ModerationFlag::Banned => self.banned = false,
            ModerationFlag::Silenced => self.silenced = false,
        }
    }

    pub fn is_flagged(&self, flag: ModerationFlag) -> bool {
        match flag {
            ModerationFlag::Banned => self.banned,
            ModerationFlag::Silenced => self.silenced,
        }
    }
}

pub trait PermissionPolicy {
    fn has_permissions(&self, requester: &AccountRole, target: &AccountRole) -> bool;
}

pub struct AdminAccountPolicy;

impl PermissionPolicy for AdminAccountPolicy {
    fn has_permissions(&self, requester: &AccountRole, target: &AccountRole) -> bool {
        matches!(requester, AccountRole::Admin) && !matches!(target, AccountRole::Admin)
    }
}

pub struct ModeratorAccountPolicy;

impl PermissionPolicy for ModeratorAccountPolicy {
    fn has_permissions(&self, requester: &AccountRole, target: &AccountRole) -> bool {
        matches!(requester, AccountRole::Admin | AccountRole::Moderator)
            && !matches!(target, AccountRole::Admin | AccountRole::Moderator)
    }
}

pub struct HigherRoleAccountPolicy;

impl PermissionPolicy for HigherRoleAccountPolicy {
    fn has_permissions(&self, requester: &AccountRole, target: &AccountRole) -> bool {
        match requester {
            AccountRole::Admin => !matches!(target, AccountRole::Admin),
            AccountRole::Moderator => matches!(target, AccountRole::User),
            AccountRole::User => false,
        }
    }
}
