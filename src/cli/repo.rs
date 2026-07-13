//! Repository management CLI handlers.

use anyhow::{Context, Result};
use std::path::Path;

use super::open_db;
use crate::git::{init_bare_repo, normalize_repo_name, repo_disk_path, validate_repo_name};

/// Creates a new repository with a bare Git repo on disk.
///
/// Uses DB-first atomicity: inserts the metadata row first, then initializes
/// the bare repo. If disk init fails, the DB entry is rolled back.
pub fn run_repo_create(data_dir: &Path, name: &str, owner: Option<&str>) -> Result<()> {
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
