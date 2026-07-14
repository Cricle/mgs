//! Repository management CLI handlers.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use super::open_db;
use crate::git::{
    check_git_commands, init_bare_repo, normalize_repo_name, repo_disk_path, validate_repo_name,
};

/// Creates a new repository with a bare Git repo on disk.
///
/// Uses DB-first atomicity: inserts the metadata row first, then initializes
/// the bare repo. If disk init fails, the DB entry is rolled back.
pub fn run_repo_create(data_dir: &Path, name: &str, owner: Option<&str>) -> Result<()> {
    check_git_commands()?;
    let name = normalize_repo_name(name);
    validate_repo_name(name)?;
    let db = open_db(data_dir)?;

    if db.find_repo(name)?.is_some() {
        anyhow::bail!("repository '{}' already exists", name);
    }

    let owner_username = match owner {
        Some(name) => name.to_string(),
        None => std::env::var("USER")
            .context("no owner specified and USER env not set; use --owner <username>")?,
    };

    let owner = db
        .find_user_by_username(&owner_username)?
        .with_context(|| format!("owner user '{}' not found", owner_username))?;

    let disk_path = repo_disk_path(data_dir, name);
    db.create_repo(name, owner.id)?;
    if let Err(e) = init_bare_repo(&disk_path) {
        let _ = db.delete_repo(name);
        return Err(e);
    }

    println!("Created repository '{}' (owner: {})", name, owner_username);
    println!();
    if let Some(token) = &owner.token {
        println!(
            "Clone via HTTP: git clone http://{}@<host>:8080/{}.git",
            token, name
        );
    }
    println!("Clone via SSH:  git clone ssh://git@<host>/{}.git", name);
    Ok(())
}

/// Lists all repositories with their owners.
pub fn run_repo_list(data_dir: &Path) -> Result<()> {
    let db = open_db(data_dir)?;
    let repos = db.list_repos()?;
    if repos.is_empty() {
        println!("No repositories found.");
        return Ok(());
    }
    for repo in &repos {
        let owner_name = db
            .find_user_by_id(repo.owner_id)?
            .map(|u| u.username)
            .unwrap_or_else(|| "unknown".to_string());
        println!("{} (owner: {})", repo.name, owner_name);
    }
    Ok(())
}

/// Removes a repository from both disk and the database.
///
/// Deletes the bare repo directory first, then the DB entry, to avoid
/// orphaned disk artifacts on partial failure.
pub fn run_repo_remove(data_dir: &Path, name: &str) -> Result<()> {
    let db = open_db(data_dir)?;
    let disk_path = repo_disk_path(data_dir, name);

    if db.find_repo(name)?.is_some() {
        if disk_path.exists() {
            std::fs::remove_dir_all(&disk_path)
                .with_context(|| format!("failed to remove {}", disk_path.display()))?;
        }
        db.delete_repo(name)?;
        println!("Removed repository '{}'", name);
    } else {
        println!("Repository '{}' not found", name);
    }
    Ok(())
}

/// Links the current git repository to a remote on the MGS server.
///
/// Constructs the remote URL based on transport type and sets it via
/// `git remote add` or `git remote set-url`.
pub fn run_repo_link(
    data_dir: &Path,
    name: &str,
    username: &str,
    host: &str,
    remote_name: &str,
    transport: &str,
) -> Result<()> {
    let name = normalize_repo_name(name);
    validate_repo_name(name)?;
    let db = open_db(data_dir)?;

    let user = db
        .find_user_by_username(username)?
        .with_context(|| format!("user '{}' not found", username))?;

    db.find_repo(name)?
        .with_context(|| format!("repository '{}' not found", name))?;

    let url = match transport {
        "http" => {
            let token = user
                .token
                .as_ref()
                .with_context(|| format!("user '{}' has no HTTP token", username))?;
            format!("http://{}@{}/{}.git", token, host, name)
        }
        "ssh" => format!("ssh://git@{}/{}.git", host, name),
        _ => anyhow::bail!("unsupported transport '{}', use 'http' or 'ssh'", transport),
    };

    // Verify current directory is a git repo
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .context("failed to run git rev-parse")?;
    if !output.status.success() {
        anyhow::bail!("not a git repository (run this command inside a git repo)");
    }

    // Check if remote already exists
    let remotes = Command::new("git")
        .args(["remote"])
        .output()
        .context("failed to list git remotes")?;
    let remotes_str = String::from_utf8_lossy(&remotes.stdout);
    let remote_exists = remotes_str.split_whitespace().any(|r| r == remote_name);

    if remote_exists {
        let status = Command::new("git")
            .args(["remote", "set-url", remote_name, &url])
            .status()
            .context("failed to set remote url")?;
        if !status.success() {
            anyhow::bail!("git remote set-url failed");
        }
        println!("Updated remote '{}' → {}", remote_name, url);
    } else {
        let status = Command::new("git")
            .args(["remote", "add", remote_name, &url])
            .status()
            .context("failed to add remote")?;
        if !status.success() {
            anyhow::bail!("git remote add failed");
        }
        println!("Added remote '{}' → {}", remote_name, url);
    }

    Ok(())
}
