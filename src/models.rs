//! Data models for MGS entities.

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
#[derive(Debug, Clone)]
pub struct Repository {
    /// Unique identifier.
    pub id: i64,
    /// Repository path (e.g. `"team/backend"`), without `.git` suffix.
    pub name: String,
    /// Owner user's ID.
    pub owner_id: i64,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
}
