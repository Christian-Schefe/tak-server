#[derive(Debug, Clone)]
pub struct Moderation {
    pub user_type: UserType,
    pub role: UserRole,
    pub flags: ModerationFlags,
}

impl Default for Moderation {
    fn default() -> Self {
        Self {
            user_type: UserType::Player,
            role: UserRole::User,
            flags: ModerationFlags::empty(),
        }
    }
}

impl Moderation {
    pub fn is_guest(&self) -> bool {
        matches!(self.user_type, UserType::Guest)
    }
    pub fn is_admin(&self) -> bool {
        matches!(self.role, UserRole::Admin)
    }
    pub fn set_banned(&mut self, is_banned: bool) {
        if is_banned {
            self.flags.set_flag(ModerationFlag::Banned);
        } else {
            self.flags.unset_flag(ModerationFlag::Banned);
        }
    }
    pub fn set_silenced(&mut self, is_silenced: bool) {
        if is_silenced {
            self.flags.set_flag(ModerationFlag::Silenced);
        } else {
            self.flags.unset_flag(ModerationFlag::Silenced);
        }
    }
    pub fn set_role(&mut self, role: UserRole) {
        self.role = role;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UserType {
    Player,
    Guest,
    Bot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRole {
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
    pub fn empty() -> Self {
        Self {
            banned: false,
            silenced: false,
        }
    }

    pub fn new(banned: bool, silenced: bool) -> Self {
        Self { banned, silenced }
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
    fn has_permissions(&self, requester: &Moderation, target: &Moderation) -> bool;
}

pub struct AdminUserPolicy;

impl PermissionPolicy for AdminUserPolicy {
    fn has_permissions(&self, requester: &Moderation, target: &Moderation) -> bool {
        matches!(requester.role, UserRole::Admin) && !matches!(target.role, UserRole::Admin)
    }
}

pub struct ModeratorUserPolicy;

impl PermissionPolicy for ModeratorUserPolicy {
    fn has_permissions(&self, requester: &Moderation, target: &Moderation) -> bool {
        matches!(requester.role, UserRole::Admin | UserRole::Moderator)
            && !matches!(target.role, UserRole::Admin | UserRole::Moderator)
    }
}

pub struct HigherRoleUserPolicy;

impl PermissionPolicy for HigherRoleUserPolicy {
    fn has_permissions(&self, requester: &Moderation, target: &Moderation) -> bool {
        match requester.role {
            UserRole::Admin => !matches!(target.role, UserRole::Admin),
            UserRole::Moderator => matches!(target.role, UserRole::User),
            UserRole::User => false,
        }
    }
}
