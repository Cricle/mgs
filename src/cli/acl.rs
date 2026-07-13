use anyhow::{Context, Result};
use std::path::Path;

use super::open_db;
use crate::models::PermLevel;

pub fn run_acl_grant(data_dir: &Path, username: &str, repo_name: &str, perm: &str) -> Result<()> {
    let level = PermLevel::parse(perm).with_context(|| {
        format!(
            "invalid permission level '{}', must be one of: read, write, admin",
            perm
        )
    })?;

    let db = open_db(data_dir)?;

    let user = db
        .find_user_by_username(username)?
        .with_context(|| format!("user '{}' not found", username))?;
    let repo = db
        .find_repo(repo_name)?
        .with_context(|| format!("repository '{}' not found", repo_name))?;

    db.grant_permission(user.id, repo.id, &level)?;
    println!(
        "Granted {} to '{}' on '{}'",
        level.as_str(),
        username,
        repo_name
    );
    Ok(())
}

pub fn run_acl_revoke(data_dir: &Path, username: &str, repo_name: &str) -> Result<()> {
    let db = open_db(data_dir)?;

    let user = db
        .find_user_by_username(username)?
        .with_context(|| format!("user '{}' not found", username))?;
    let repo = db
        .find_repo(repo_name)?
        .with_context(|| format!("repository '{}' not found", repo_name))?;

    if db.revoke_permission(user.id, repo.id)? {
        println!("Revoked permissions from '{}' on '{}'", username, repo_name);
    } else {
        println!("No permissions found for '{}' on '{}'", username, repo_name);
    }
    Ok(())
}

pub fn run_acl_list(data_dir: &Path, repo_name: &str) -> Result<()> {
    let db = open_db(data_dir)?;
    let repo = db
        .find_repo(repo_name)?
        .with_context(|| format!("repository '{}' not found", repo_name))?;

    let owner = db
        .find_user_by_id(repo.owner_id)?
        .map(|u| u.username)
        .unwrap_or_else(|| "unknown".to_string());

    println!("Repository: {} (owner: {})", repo_name, owner);

    let perms = db.list_permissions(repo.id)?;
    if perms.is_empty() {
        println!("No additional permissions granted.");
        return Ok(());
    }
    for (user, level) in &perms {
        println!("  {} — {}", user.username, level.as_str());
    }
    Ok(())
}
