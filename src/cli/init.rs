//! `mgs init` — initializes the data directory and database.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::db::Database;

/// Creates the data directory structure and opens (or creates) the database.
///
/// Idempotent: prints a message and returns `Ok(())` if already initialized.
pub fn run_init(data_dir: &Path) -> Result<()> {
    let repos_dir = data_dir.join("repos");
    fs::create_dir_all(&repos_dir)
        .with_context(|| format!("failed to create {}", repos_dir.display()))?;

    let db_path = data_dir.join("mgs.db");
    if db_path.exists() {
        println!("mgs already initialized at {}", data_dir.display());
        return Ok(());
    }

    Database::open(&db_path)?;
    println!("Initialized mgs in {}", data_dir.display());
    Ok(())
}
