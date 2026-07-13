#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermLevel {
    Read,
    Write,
    Admin,
}

impl PermLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            PermLevel::Read => "read",
            PermLevel::Write => "write",
            PermLevel::Admin => "admin",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "read" => Some(PermLevel::Read),
            "write" => Some(PermLevel::Write),
            "admin" => Some(PermLevel::Admin),
            _ => None,
        }
    }

    /// Returns true if `self` grants at least `required` level.
    pub fn satisfies(&self, required: &PermLevel) -> bool {
        match (self, required) {
            (PermLevel::Admin, _) => true,
            (PermLevel::Write, PermLevel::Write) => true,
            (PermLevel::Write, PermLevel::Read) => true,
            (PermLevel::Read, PermLevel::Read) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct SshKey {
    pub id: i64,
    pub user_id: i64,
    pub key_type: String,
    pub public_key: String,
    pub fingerprint: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Repository {
    pub id: i64,
    pub name: String,
    pub owner_id: i64,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Permission {
    pub id: i64,
    pub user_id: i64,
    pub repo_id: i64,
    pub level: PermLevel,
}
