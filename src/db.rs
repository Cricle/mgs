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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_db() -> (TempDir, Database) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        (tmp, db)
    }

    // --- Users ---

    #[test]
    fn test_create_and_find_user() {
        let (_tmp, db) = test_db();
        let user = db.create_user("alice").unwrap();
        assert_eq!(user.username, "alice");
        assert!(user.id > 0);
        assert!(!user.created_at.is_empty());

        let found = db.find_user_by_username("alice").unwrap().unwrap();
        assert_eq!(found.id, user.id);
        assert_eq!(found.username, "alice");
    }

    #[test]
    fn test_find_user_by_id() {
        let (_tmp, db) = test_db();
        let user = db.create_user("bob").unwrap();
        let found = db.find_user_by_id(user.id).unwrap().unwrap();
        assert_eq!(found.username, "bob");
    }

    #[test]
    fn test_find_user_not_found() {
        let (_tmp, db) = test_db();
        assert!(db.find_user_by_username("nobody").unwrap().is_none());
        assert!(db.find_user_by_id(999).unwrap().is_none());
    }

    #[test]
    fn test_create_user_duplicate() {
        let (_tmp, db) = test_db();
        db.create_user("alice").unwrap();
        assert!(db.create_user("alice").is_err());
    }

    #[test]
    fn test_list_users() {
        let (_tmp, db) = test_db();
        assert!(db.list_users().unwrap().is_empty());

        db.create_user("charlie").unwrap();
        db.create_user("alice").unwrap();
        db.create_user("bob").unwrap();

        let users = db.list_users().unwrap();
        assert_eq!(users.len(), 3);
        assert_eq!(users[0].username, "alice"); // ordered by username
        assert_eq!(users[1].username, "bob");
        assert_eq!(users[2].username, "charlie");
    }

    #[test]
    fn test_delete_user() {
        let (_tmp, db) = test_db();
        db.create_user("alice").unwrap();
        assert!(db.delete_user("alice").unwrap());
        assert!(!db.delete_user("alice").unwrap());
        assert!(db.find_user_by_username("alice").unwrap().is_none());
    }

    #[test]
    fn test_delete_user_cascades_keys() {
        let (_tmp, db) = test_db();
        let user = db.create_user("alice").unwrap();
        db.add_ssh_key(user.id, "ssh-ed25519", "AAAA1234", "SHA256:abc")
            .unwrap();

        db.delete_user("alice").unwrap();
        assert!(db.list_ssh_keys(user.id).unwrap().is_empty());
    }

    // --- SSH Keys ---

    #[test]
    fn test_add_and_list_keys() {
        let (_tmp, db) = test_db();
        let user = db.create_user("alice").unwrap();

        let k1 = db
            .add_ssh_key(user.id, "ssh-ed25519", "AAAA111", "SHA256:aaa")
            .unwrap();
        let k2 = db
            .add_ssh_key(user.id, "ssh-rsa", "AAAA222", "SHA256:bbb")
            .unwrap();

        assert_eq!(k1.user_id, user.id);
        assert_eq!(k1.key_type, "ssh-ed25519");
        assert_eq!(k2.key_type, "ssh-rsa");

        let keys = db.list_ssh_keys(user.id).unwrap();
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].fingerprint, "SHA256:aaa");
        assert_eq!(keys[1].fingerprint, "SHA256:bbb");
    }

    #[test]
    fn test_list_keys_empty() {
        let (_tmp, db) = test_db();
        let user = db.create_user("alice").unwrap();
        assert!(db.list_ssh_keys(user.id).unwrap().is_empty());
    }

    #[test]
    fn test_delete_key() {
        let (_tmp, db) = test_db();
        let user = db.create_user("alice").unwrap();
        db.add_ssh_key(user.id, "ssh-ed25519", "AAAA", "SHA256:x")
            .unwrap();

        assert!(db.delete_ssh_key("SHA256:x").unwrap());
        assert!(!db.delete_ssh_key("SHA256:x").unwrap());
        assert!(db.list_ssh_keys(user.id).unwrap().is_empty());
    }

    #[test]
    fn test_add_key_duplicate_fingerprint() {
        let (_tmp, db) = test_db();
        let user = db.create_user("alice").unwrap();
        db.add_ssh_key(user.id, "ssh-ed25519", "AAAA", "SHA256:dup")
            .unwrap();
        assert!(
            db.add_ssh_key(user.id, "ssh-rsa", "BBBB", "SHA256:dup")
                .is_err()
        );
    }

    #[test]
    fn test_find_user_by_fingerprint() {
        let (_tmp, db) = test_db();
        let user = db.create_user("alice").unwrap();
        db.add_ssh_key(user.id, "ssh-ed25519", "AAAA", "SHA256:findme")
            .unwrap();

        let found = db
            .find_user_by_fingerprint("SHA256:findme")
            .unwrap()
            .unwrap();
        assert_eq!(found.username, "alice");
        assert!(
            db.find_user_by_fingerprint("SHA256:nope")
                .unwrap()
                .is_none()
        );
    }

    // --- Repositories ---

    #[test]
    fn test_create_and_find_repo() {
        let (_tmp, db) = test_db();
        let user = db.create_user("alice").unwrap();
        let repo = db.create_repo("team/project", user.id).unwrap();

        assert_eq!(repo.name, "team/project");
        assert_eq!(repo.owner_id, user.id);

        let found = db.find_repo("team/project").unwrap().unwrap();
        assert_eq!(found.id, repo.id);
    }

    #[test]
    fn test_find_repo_not_found() {
        let (_tmp, db) = test_db();
        assert!(db.find_repo("nonexistent").unwrap().is_none());
    }

    #[test]
    fn test_create_repo_duplicate() {
        let (_tmp, db) = test_db();
        let user = db.create_user("alice").unwrap();
        db.create_repo("myrepo", user.id).unwrap();
        assert!(db.create_repo("myrepo", user.id).is_err());
    }

    #[test]
    fn test_list_repos() {
        let (_tmp, db) = test_db();
        let user = db.create_user("alice").unwrap();
        assert!(db.list_repos().unwrap().is_empty());

        db.create_repo("beta", user.id).unwrap();
        db.create_repo("alpha", user.id).unwrap();
        db.create_repo("gamma", user.id).unwrap();

        let repos = db.list_repos().unwrap();
        assert_eq!(repos.len(), 3);
        assert_eq!(repos[0].name, "alpha"); // ordered by name
        assert_eq!(repos[1].name, "beta");
        assert_eq!(repos[2].name, "gamma");
    }

    #[test]
    fn test_delete_repo() {
        let (_tmp, db) = test_db();
        let user = db.create_user("alice").unwrap();
        db.create_repo("myrepo", user.id).unwrap();

        assert!(db.delete_repo("myrepo").unwrap());
        assert!(!db.delete_repo("myrepo").unwrap());
        assert!(db.find_repo("myrepo").unwrap().is_none());
    }

    // --- Open ---

    #[test]
    fn test_open_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let db1 = Database::open(&db_path).unwrap();
        db1.create_user("alice").unwrap();
        drop(db1);

        let db2 = Database::open(&db_path).unwrap();
        let user = db2.find_user_by_username("alice").unwrap().unwrap();
        assert_eq!(user.username, "alice");
    }
}
