//! Data models for MGS entities.

/// Permission level for repository access.
///
/// Hierarchy: `Admin` > `Write` > `Read`. Each level implies all lower levels.
/// Repository owners implicitly have `Admin` without an explicit permissions row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermLevel {
    /// Clone and fetch only.
    Read,
    /// Push (implies read).
    Write,
    /// Push + manage permissions (implies write).
    Admin,
}

impl PermLevel {
    /// Returns the lowercase string representation (`"read"`, `"write"`, `"admin"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            PermLevel::Read => "read",
            PermLevel::Write => "write",
            PermLevel::Admin => "admin",
        }
    }

    /// Parses a permission level from a string.
    ///
    /// Accepts `"read"`, `"write"`, or `"admin"` (case-sensitive).
    /// Returns an error for any other input.
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

    /// Returns `true` if `self` grants at least the `required` level.
    ///
    /// - `Admin` satisfies any requirement
    /// - `Write` satisfies `Write` and `Read`
    /// - `Read` satisfies only `Read`
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

/// A registered user who can access repositories via SSH.
#[derive(Debug, Clone)]
pub struct User {
    /// Unique identifier.
    pub id: i64,
    /// Login name (alphanumeric, `_`, `-` only).
    pub username: String,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
}

/// An SSH public key associated with a user.
///
/// Users can have multiple keys. The `fingerprint` is used by `mgs-ssh`
/// to identify the connecting user.
#[derive(Debug, Clone)]
pub struct SshKey {
    /// Unique identifier.
    pub id: i64,
    /// Owning user's ID.
    pub user_id: i64,
    /// Key type (e.g. `ssh-ed25519`, `ssh-rsa`).
    pub key_type: String,
    /// Base64-encoded public key data.
    pub public_key: String,
    /// SHA256 fingerprint from `ssh-keygen -lf`.
    pub fingerprint: String,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
}

/// A Git repository.
///
/// The `owner_id` user implicitly has `Admin` permission without
/// an explicit row in the `permissions` table.
#[derive(Debug, Clone)]
pub struct Repository {
    /// Unique identifier.
    pub id: i64,
    /// Repository path (e.g. `"team/backend"`), without `.git` suffix.
    pub name: String,
    /// Owner user's ID (implicit admin).
    pub owner_id: i64,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
}

/// An explicit permission grant linking a user to a repository.
///
/// Note: repository owners have implicit `Admin` and do not need a row here.
#[derive(Debug, Clone)]
pub struct Permission {
    /// Unique identifier.
    pub id: i64,
    /// User ID.
    pub user_id: i64,
    /// Repository ID.
    pub repo_id: i64,
    /// Granted permission level.
    pub level: PermLevel,
}
