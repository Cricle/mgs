pub mod acl;
pub mod init;
pub mod repo;
pub mod user;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mgs", about = "Mini Git Server")]
pub struct Cli {
    /// Data directory (default: ~/.mgs)
    #[arg(long, env = "MGS_HOME")]
    pub data_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

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
    pub fn data_dir(&self) -> PathBuf {
        self.data_dir.clone().unwrap_or_else(|| {
            let home = std::env::var("HOME").expect("HOME not set");
            PathBuf::from(home).join(".mgs")
        })
    }
}
