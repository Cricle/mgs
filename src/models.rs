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

    pub fn parse(s: &str) -> anyhow::Result<Self> {
        match s {
            "read" => Ok(PermLevel::Read),
            "write" => Ok(PermLevel::Write),
            "admin" => Ok(PermLevel::Admin),
            _ => anyhow::bail!(
                "invalid permission level '{}', expected one of: read, write, admin",
                s
            ),
        }
    }

    /// Returns true if `self` grants at least `required` level.
    pub fn satisfies(&self, required: &PermLevel) -> bool {
        matches!(
            (self, required),
            (PermLevel::Admin, _)
                | (PermLevel::Write, PermLevel::Write)
                | (PermLevel::Write, PermLevel::Read)
                | (PermLevel::Read, PermLevel::Read)
        )
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
