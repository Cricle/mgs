//! User and SSH key management CLI handlers.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use super::open_db;
use crate::auth::{compute_fingerprint, parse_ssh_public_key};
use crate::git::validate_username;

/// Reads an SSH public key file and returns `(key_type, public_key, fingerprint)`.
fn read_ssh_key(key_path: &Path) -> Result<(String, String, String)> {
    let content = fs::read_to_string(key_path)
        .with_context(|| format!("failed to read key file: {}", key_path.display()))?;
    let (key_type, public_key) = parse_ssh_public_key(&content)?;
    let fingerprint = compute_fingerprint(&content)?;
    Ok((key_type, public_key, fingerprint))
}

/// Creates a new user with an SSH public key.
///
/// Validates the username, reads and parses the key file, computes its fingerprint,
/// then inserts both the user and key into the database.
pub fn run_user_add(data_dir: &Path, username: &str, key_path: &Path) -> Result<()> {
    validate_username(username)?;
    let db = open_db(data_dir)?;

    if db.find_user_by_username(username)?.is_some() {
        anyhow::bail!("user '{}' already exists", username);
    }

    let (key_type, public_key, fingerprint) = read_ssh_key(key_path)?;

    let user = db.create_user(username)?;
    db.add_ssh_key(user.id, &key_type, &public_key, &fingerprint)?;

    println!(
        "Created user '{}' with key fingerprint {}",
        username, fingerprint
    );
    Ok(())
}

/// Lists all registered users with their SSH key counts.
pub fn run_user_list(data_dir: &Path) -> Result<()> {
    let db = open_db(data_dir)?;
    let users = db.list_users()?;
    if users.is_empty() {
        println!("No users found.");
        return Ok(());
    }
    for user in &users {
        let keys = db.list_ssh_keys(user.id)?;
        println!("{} ({} keys)", user.username, keys.len());
    }
    Ok(())
}

/// Removes a user and all associated SSH keys and permission grants.
pub fn run_user_remove(data_dir: &Path, username: &str) -> Result<()> {
    let db = open_db(data_dir)?;
    if db.delete_user(username)? {
        println!("Removed user '{}'", username);
    } else {
        println!("User '{}' not found", username);
    }
    Ok(())
}

/// Adds an additional SSH key to an existing user.
pub fn run_key_add(data_dir: &Path, username: &str, key_path: &Path) -> Result<()> {
    let db = open_db(data_dir)?;
    let user = db
        .find_user_by_username(username)?
        .with_context(|| format!("user '{}' not found", username))?;

    let (key_type, public_key, fingerprint) = read_ssh_key(key_path)?;

    db.add_ssh_key(user.id, &key_type, &public_key, &fingerprint)?;
    println!("Added key {} to user '{}'", fingerprint, username);
    Ok(())
}

/// Lists all SSH keys for a user, showing type, fingerprint, and public key.
pub fn run_key_list(data_dir: &Path, username: &str) -> Result<()> {
    let db = open_db(data_dir)?;
    let user = db
        .find_user_by_username(username)?
        .with_context(|| format!("user '{}' not found", username))?;

    let keys = db.list_ssh_keys(user.id)?;
    if keys.is_empty() {
        println!("No keys for user '{}'", username);
        return Ok(());
    }
    for key in &keys {
        println!("{} {} {}", key.key_type, key.fingerprint, key.public_key);
    }
    Ok(())
}

/// Removes an SSH key by its SHA256 fingerprint.
pub fn run_key_remove(data_dir: &Path, fingerprint: &str) -> Result<()> {
    let db = open_db(data_dir)?;
    if db.delete_ssh_key(fingerprint)? {
        println!("Removed key {}", fingerprint);
    } else {
        println!("Key {} not found", fingerprint);
    }
    Ok(())
}
