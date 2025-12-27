#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountRole {
    User,
    Moderator,
    Admin,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AccountFlag {
    Banned,
    Silenced,
    Bot,
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
