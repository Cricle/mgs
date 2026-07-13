use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::db::Database;

pub fn run_init(data_dir: &PathBuf) -> Result<()> {
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
