//! SQLite database layer.
//!
//! Provides the [`Database`] struct with methods for managing users,
//! SSH keys, and repositories. Uses WAL journal mode and foreign key
//! constraints.

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::path::Path;

use crate::models::{Repository, SshKey, User};

/// MGS metadata database backed by SQLite.
///
/// Opens (or creates) a SQLite database at the given path and applies
/// the schema migration. Uses WAL mode for concurrent read access.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Opens or creates the database at `db_path`.
    ///
    /// Enables WAL journal mode and foreign keys, then runs the
    /// schema migration (`migrations/001_init.sql`).
    pub fn open(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("failed to open database: {}", db_path.display()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let schema = include_str!("../migrations/001_init.sql");
        conn.execute_batch(schema)?;
        Ok(Self { conn })
    }

    // --- Users ---

    /// Creates a new user with the given `username`.
    ///
    /// Returns the created [`User`] with its assigned `id` and `created_at`.
    /// Fails if `username` already exists (UNIQUE constraint).
    pub fn create_user(&self, username: &str) -> Result<User> {
        self.conn.execute(
            "INSERT INTO users (username) VALUES (?1)",
            params![username],
        )?;
        let id = self.conn.last_insert_rowid();
        let user = self.conn.query_row(
            "SELECT id, username, created_at FROM users WHERE id = ?1",
            params![id],
            |row| {
                Ok(User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    created_at: row.get(2)?,
                })
            },
        )?;
        Ok(user)
    }

    /// Finds a user by their exact username.
    ///
    /// Returns `None` if no user with that name exists.
    pub fn find_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, username, created_at FROM users WHERE username = ?1")?;
        let mut rows = stmt.query_map(params![username], |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Finds a user by their numeric ID.
    pub fn find_user_by_id(&self, id: i64) -> Result<Option<User>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, username, created_at FROM users WHERE id = ?1")?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Finds the user who owns the SSH key with the given `fingerprint`.
    ///
    /// Used by `mgs-ssh` to identify the connecting user from the
    /// fingerprint passed via `authorized_keys`.
    pub fn find_user_by_fingerprint(&self, fingerprint: &str) -> Result<Option<User>> {
        let mut stmt = self.conn.prepare(
            "SELECT u.id, u.username, u.created_at FROM users u
             JOIN ssh_keys k ON k.user_id = u.id
             WHERE k.fingerprint = ?1",
        )?;
        let mut rows = stmt.query_map(params![fingerprint], |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Lists all users, ordered by username.
    pub fn list_users(&self) -> Result<Vec<User>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, username, created_at FROM users ORDER BY username")?;
        let rows = stmt.query_map([], |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Deletes a user by username. Returns `true` if a row was removed.
    ///
    /// Cascades to delete all associated SSH keys and permission grants.
    pub fn delete_user(&self, username: &str) -> Result<bool> {
        let n = self
            .conn
            .execute("DELETE FROM users WHERE username = ?1", params![username])?;
        Ok(n > 0)
    }

    // --- SSH Keys ---

    /// Adds an SSH public key for a user.
    ///
    /// The `fingerprint` should be computed via [`crate::auth::compute_fingerprint`].
    /// Fails if the `public_key` or `fingerprint` already exists (UNIQUE constraints).
    pub fn add_ssh_key(
        &self,
        user_id: i64,
        key_type: &str,
        public_key: &str,
        fingerprint: &str,
    ) -> Result<SshKey> {
        self.conn.execute(
            "INSERT INTO ssh_keys (user_id, key_type, public_key, fingerprint) VALUES (?1, ?2, ?3, ?4)",
            params![user_id, key_type, public_key, fingerprint],
        )?;
        let id = self.conn.last_insert_rowid();
        let key = self.conn.query_row(
            "SELECT id, user_id, key_type, public_key, fingerprint, created_at FROM ssh_keys WHERE id = ?1",
            params![id],
            |row| {
                Ok(SshKey {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    key_type: row.get(2)?,
                    public_key: row.get(3)?,
                    fingerprint: row.get(4)?,
                    created_at: row.get(5)?,
                })
            },
        )?;
        Ok(key)
    }

    /// Lists all SSH keys for a user, ordered by ID.
    pub fn list_ssh_keys(&self, user_id: i64) -> Result<Vec<SshKey>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, key_type, public_key, fingerprint, created_at
             FROM ssh_keys WHERE user_id = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map(params![user_id], |row| {
            Ok(SshKey {
                id: row.get(0)?,
                user_id: row.get(1)?,
                key_type: row.get(2)?,
                public_key: row.get(3)?,
                fingerprint: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Deletes an SSH key by its SHA256 fingerprint. Returns `true` if removed.
    pub fn delete_ssh_key(&self, fingerprint: &str) -> Result<bool> {
        let n = self.conn.execute(
            "DELETE FROM ssh_keys WHERE fingerprint = ?1",
            params![fingerprint],
        )?;
        Ok(n > 0)
    }

    // --- Repositories ---

    /// Creates a new repository with the given `name` and `owner_id`.
    ///
    /// The name should be normalized (no `.git` suffix) before calling this.
    /// Fails if a repository with that name already exists.
    pub fn create_repo(&self, name: &str, owner_id: i64) -> Result<Repository> {
        self.conn.execute(
            "INSERT INTO repositories (name, owner_id) VALUES (?1, ?2)",
            params![name, owner_id],
        )?;
        let id = self.conn.last_insert_rowid();
        let repo = self.conn.query_row(
            "SELECT id, name, owner_id, created_at FROM repositories WHERE id = ?1",
            params![id],
            |row| {
                Ok(Repository {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    owner_id: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )?;
        Ok(repo)
    }

    /// Finds a repository by its exact name.
    pub fn find_repo(&self, name: &str) -> Result<Option<Repository>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, owner_id, created_at FROM repositories WHERE name = ?1")?;
        let mut rows = stmt.query_map(params![name], |row| {
            Ok(Repository {
                id: row.get(0)?,
                name: row.get(1)?,
                owner_id: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Lists all repositories, ordered by name.
    pub fn list_repos(&self) -> Result<Vec<Repository>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, owner_id, created_at FROM repositories ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            Ok(Repository {
                id: row.get(0)?,
                name: row.get(1)?,
                owner_id: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Deletes a repository by name. Returns `true` if removed.
    ///
    /// Note: does NOT delete the bare repo on disk. The caller must
    /// handle disk cleanup separately.
    pub fn delete_repo(&self, name: &str) -> Result<bool> {
        let n = self
            .conn
            .execute("DELETE FROM repositories WHERE name = ?1", params![name])?;
        Ok(n > 0)
    }
}
