use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::path::Path;

use crate::models::{PermLevel, Repository, SshKey, User};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("failed to open database: {}", db_path.display()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let schema = include_str!("../migrations/001_init.sql");
        conn.execute_batch(schema)?;
        Ok(Self { conn })
    }

    // --- Users ---

    pub fn create_user(&self, username: &str) -> Result<User> {
        self.conn
            .execute("INSERT INTO users (username) VALUES (?1)", params![username])?;
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

    pub fn find_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, username, created_at FROM users WHERE username = ?1",
        )?;
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

    pub fn delete_user(&self, username: &str) -> Result<bool> {
        let n = self
            .conn
            .execute("DELETE FROM users WHERE username = ?1", params![username])?;
        Ok(n > 0)
    }

    // --- SSH Keys ---

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

    pub fn delete_ssh_key(&self, fingerprint: &str) -> Result<bool> {
        let n = self.conn.execute(
            "DELETE FROM ssh_keys WHERE fingerprint = ?1",
            params![fingerprint],
        )?;
        Ok(n > 0)
    }

    // --- Repositories ---

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

    pub fn find_repo(&self, name: &str) -> Result<Option<Repository>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, owner_id, created_at FROM repositories WHERE name = ?1",
        )?;
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

    pub fn delete_repo(&self, name: &str) -> Result<bool> {
        let n = self
            .conn
            .execute("DELETE FROM repositories WHERE name = ?1", params![name])?;
        Ok(n > 0)
    }

    // --- Permissions ---

    pub fn grant_permission(&self, user_id: i64, repo_id: i64, level: &PermLevel) -> Result<()> {
        self.conn.execute(
            "INSERT INTO permissions (user_id, repo_id, level) VALUES (?1, ?2, ?3)
             ON CONFLICT(user_id, repo_id) DO UPDATE SET level = excluded.level",
            params![user_id, repo_id, level.as_str()],
        )?;
        Ok(())
    }

    pub fn revoke_permission(&self, user_id: i64, repo_id: i64) -> Result<bool> {
        let n = self.conn.execute(
            "DELETE FROM permissions WHERE user_id = ?1 AND repo_id = ?2",
            params![user_id, repo_id],
        )?;
        Ok(n > 0)
    }

    /// Get effective permission for a user on a repo.
    /// Returns None if user has no access (and is not owner).
    pub fn get_permission(&self, user_id: i64, repo_id: i64) -> Result<Option<PermLevel>> {
        // Check if owner first
        let is_owner = self.conn.query_row(
            "SELECT 1 FROM repositories WHERE id = ?1 AND owner_id = ?2",
            params![repo_id, user_id],
            |_| Ok(true),
        ).unwrap_or(false);

        if is_owner {
            return Ok(Some(PermLevel::Admin));
        }

        let mut stmt = self
            .conn
            .prepare("SELECT level FROM permissions WHERE user_id = ?1 AND repo_id = ?2")?;
        let mut rows = stmt.query_map(params![user_id, repo_id], |row| {
            let level_str: String = row.get(0)?;
            Ok(level_str)
        })?;
        match rows.next() {
            Some(row) => Ok(PermLevel::from_str(&row?)),
            None => Ok(None),
        }
    }

    pub fn list_permissions(&self, repo_id: i64) -> Result<Vec<(User, PermLevel)>> {
        let mut stmt = self.conn.prepare(
            "SELECT u.id, u.username, u.created_at, p.level
             FROM permissions p JOIN users u ON u.id = p.user_id
             WHERE p.repo_id = ?1 ORDER BY u.username",
        )?;
        let rows = stmt.query_map(params![repo_id], |row| {
            let user = User {
                id: row.get(0)?,
                username: row.get(1)?,
                created_at: row.get(2)?,
            };
            let level_str: String = row.get(3)?;
            Ok((user, PermLevel::from_str(&level_str).unwrap_or(PermLevel::Read)))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}
