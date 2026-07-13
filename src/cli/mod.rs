//! CLI interface for the `mgs` administration tool.
//!
//! Defines the clap-based command hierarchy and dispatches to subcommand
//! handlers in [`user`], [`repo`], [`acl`], and [`init`] modules.

pub mod acl;
pub mod init;
pub mod repo;
pub mod user;

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

use crate::db::Database;

/// Opens the MGS database in the given data directory.
pub(crate) fn open_db(data_dir: &Path) -> anyhow::Result<Database> {
    let db_path = data_dir.join("mgs.db");
    Database::open(&db_path)
}

/// Top-level CLI definition parsed by clap.
#[derive(Parser)]
#[command(name = "mgs", about = "Mini Git Server")]
pub struct Cli {
    /// Data directory (default: ~/.mgs)
    #[arg(long, env = "MGS_HOME")]
    pub data_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

/// Available top-level subcommands.
#[derive(Subcommand)]
pub enum Command {
    /// Initialize mgs data directory and database
    Init,
    /// Manage users
    User {
        #[command(subcommand)]
        command: UserCommand,
    },
    /// Manage repositories
    Repo {
        #[command(subcommand)]
        command: RepoCommand,
    },
    /// Manage access control
    Acl {
        #[command(subcommand)]
        command: AclCommand,
    },
}

/// User management subcommands.
#[derive(Subcommand)]
pub enum UserCommand {
    /// Add a new user with an SSH public key
    Add {
        username: String,
        /// Path to SSH public key file
        #[arg(long)]
        key: PathBuf,
    },
    /// List all users
    List,
    /// Remove a user
    Remove { username: String },
    /// Manage user SSH keys
    Key {
        #[command(subcommand)]
        command: KeyCommand,
    },
}

/// SSH key management subcommands (nested under `user`).
#[derive(Subcommand)]
pub enum KeyCommand {
    /// Add an SSH key to a user
    Add {
        username: String,
        #[arg(long)]
        key: PathBuf,
    },
    /// List SSH keys for a user
    List { username: String },
    /// Remove an SSH key by fingerprint
    Remove { fingerprint: String },
}

/// Repository management subcommands.
#[derive(Subcommand)]
pub enum RepoCommand {
    /// Create a new repository
    Create {
        name: String,
        /// Owner username (default: current system user)
        #[arg(long)]
        owner: Option<String>,
    },
    /// List all repositories
    List,
    /// Remove a repository
    Remove { name: String },
}

/// Access control (permission) management subcommands.
#[derive(Subcommand)]
pub enum AclCommand {
    /// Grant permission to a user on a repository
    Grant {
        username: String,
        repo: String,
        /// Permission level: read, write, admin
        #[arg(long)]
        perm: String,
    },
    /// Revoke permission from a user on a repository
    Revoke { username: String, repo: String },
    /// List permissions for a repository
    List { repo: String },
}

impl Cli {
    /// Returns the resolved data directory path.
    ///
    /// Uses the `--data-dir` flag if provided, otherwise falls back to `$MGS_HOME`,
    /// and finally `$HOME/.mgs`.
    pub fn data_dir(&self) -> PathBuf {
        self.data_dir.clone().unwrap_or_else(|| {
            let home = std::env::var("HOME").expect("HOME not set");
            PathBuf::from(home).join(".mgs")
        })
    }
}
