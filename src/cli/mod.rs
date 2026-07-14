//! CLI interface for the `mgs` administration tool.
//!
//! Defines the clap-based command hierarchy and dispatches to subcommand
//! handlers in [`user`] and [`repo`] modules.

pub mod repo;
pub mod user;

use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

use crate::db::Database;

/// Ensures the data directory exists and opens the database.
///
/// Creates `data_dir` and `data_dir/repos` if they don't exist (idempotent).
pub(crate) fn open_db(data_dir: &Path) -> anyhow::Result<Database> {
    let repos_dir = data_dir.join("repos");
    std::fs::create_dir_all(&repos_dir)
        .with_context(|| format!("failed to create {}", repos_dir.display()))?;
    let db_path = data_dir.join("mgs.db");
    Database::open(&db_path)
}

/// Top-level CLI definition parsed by clap.
#[derive(Parser)]
#[command(name = "mgs", about = "Mini Git Server")]
pub struct Cli {
    /// Data directory (default: binary directory)
    #[arg(long, env = "MGS_HOME")]
    pub data_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

/// Available top-level subcommands.
#[derive(Subcommand)]
pub enum Command {
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
    /// Start HTTP server for Git Smart HTTP protocol
    Serve {
        /// Address to bind (default: 0.0.0.0:8080)
        #[arg(long, default_value = "0.0.0.0:8080")]
        bind: String,
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
    /// Manage user authentication tokens
    Token {
        #[command(subcommand)]
        command: TokenCommand,
    },
}

/// Token management subcommands (nested under `user`).
#[derive(Subcommand)]
pub enum TokenCommand {
    /// Show a user's token
    Show { username: String },
    /// Regenerate a user's token
    Regenerate { username: String },
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
    /// Link current git repo to a remote
    Link {
        /// Repository name
        name: String,
        /// Username for token lookup
        #[arg(long)]
        user: String,
        /// Server address (host:port)
        #[arg(long)]
        host: String,
        /// Remote name
        #[arg(long, default_value = "origin")]
        remote: String,
        /// Transport: http or ssh
        #[arg(long, default_value = "http")]
        transport: String,
    },
}

impl Cli {
    /// Returns the resolved data directory path.
    ///
    /// Uses the `--data-dir` flag if provided, otherwise falls back to `$MGS_HOME`,
    /// and finally the directory containing the `mgs` binary.
    pub fn data_dir(&self) -> PathBuf {
        self.data_dir.clone().unwrap_or_else(|| {
            let exe = std::env::current_exe().expect("failed to determine executable path");
            exe.parent()
                .expect("failed to determine executable directory")
                .to_path_buf()
        })
    }
}
