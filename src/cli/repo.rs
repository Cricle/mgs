use anyhow::{Context, Result};
use std::path::Path;

use crate::db::Database;
use crate::git::{init_bare_repo, repo_disk_path, validate_repo_name};

fn open_db(data_dir: &Path) -> Result<Database> {
    let db_path = data_dir.join("mgs.db");
    Database::open(&db_path)
}

pub fn run_repo_create(data_dir: &Path, name: &str, owner: Option<&str>) -> Result<()> {
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
        .with_context(|| format!("owner user '{}' not found", &owner_username))?;

    let disk_path = repo_disk_path(data_dir, name);
    init_bare_repo(&disk_path)?;
    db.create_repo(name, owner.id)?;

    println!("Created repository '{}' (owner: {})", name, owner_username);
    Ok(())
}

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

pub fn run_repo_remove(data_dir: &Path, name: &str) -> Result<()> {
    let db = open_db(data_dir)?;
    let disk_path = repo_disk_path(data_dir, name);

    if db.delete_repo(name)? {
        if disk_path.exists() {
            std::fs::remove_dir_all(&disk_path)
                .with_context(|| format!("failed to remove {}", disk_path.display()))?;
        }
        println!("Removed repository '{}'", name);
    } else {
        println!("Repository '{}' not found", name);
    }
    Ok(())
}
