//! Configuration management CLI handlers.
//!
//! Stores key-value config in `<data_dir>/config` (simple `key=value` format).
//! Keys used by MGS:
//! - `http.host` — HTTP server address (e.g., `myserver:8080`)
//! - `ssh.host` — SSH server address (e.g., `myserver:22`)
//! - `default.user` — default username for repo link

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn config_path(data_dir: &Path) -> PathBuf {
    data_dir.join("config")
}

/// Reads all config entries from disk.
pub fn read_config(data_dir: &Path) -> Result<BTreeMap<String, String>> {
    let path = config_path(data_dir);
    let mut map = BTreeMap::new();
    if !path.exists() {
        return Ok(map);
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    Ok(map)
}

/// Writes all config entries to disk.
fn write_config(data_dir: &Path, map: &BTreeMap<String, String>) -> Result<()> {
    let path = config_path(data_dir);
    let content: String = map.iter().map(|(k, v)| format!("{}={}\n", k, v)).collect();
    fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

/// Gets a single config value.
pub fn get_config_value(data_dir: &Path, key: &str) -> Result<Option<String>> {
    let map = read_config(data_dir)?;
    Ok(map.get(key).cloned())
}

/// Sets a configuration value.
pub fn run_config_set(data_dir: &Path, key: &str, value: &str) -> Result<()> {
    validate_config_key(key)?;
    let mut map = read_config(data_dir)?;
    map.insert(key.to_string(), value.to_string());
    write_config(data_dir, &map)?;
    println!("{}={}", key, value);
    Ok(())
}

/// Gets and prints a configuration value.
pub fn run_config_get(data_dir: &Path, key: &str) -> Result<()> {
    let map = read_config(data_dir)?;
    match map.get(key) {
        Some(value) => println!("{}", value),
        None => println!("(not set)"),
    }
    Ok(())
}

/// Lists all configuration values.
pub fn run_config_list(data_dir: &Path) -> Result<()> {
    let map = read_config(data_dir)?;
    if map.is_empty() {
        println!("No configuration set.");
        println!();
        println!("Common keys:");
        println!("  http.host     HTTP server address (e.g., myserver:8080)");
        println!("  ssh.host      SSH server address (e.g., myserver:22)");
        println!("  default.user  Default username for repo link");
        return Ok(());
    }
    for (key, value) in &map {
        println!("{}={}", key, value);
    }
    Ok(())
}

fn validate_config_key(key: &str) -> Result<()> {
    let valid_keys = ["http.host", "ssh.host", "default.user"];
    if !valid_keys.contains(&key) {
        anyhow::bail!(
            "unknown config key '{}'. Valid keys: {}",
            key,
            valid_keys.join(", ")
        );
    }
    Ok(())
}
