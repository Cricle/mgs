use anyhow::{bail, Result};
use std::path::PathBuf;

pub fn run_user_add(_data_dir: &PathBuf, _username: &str, _key: &PathBuf) -> Result<()> {
    bail!("not yet implemented")
}

pub fn run_user_list(_data_dir: &PathBuf) -> Result<()> {
    bail!("not yet implemented")
}

pub fn run_user_remove(_data_dir: &PathBuf, _username: &str) -> Result<()> {
    bail!("not yet implemented")
}

pub fn run_key_add(_data_dir: &PathBuf, _username: &str, _key: &PathBuf) -> Result<()> {
    bail!("not yet implemented")
}

pub fn run_key_list(_data_dir: &PathBuf, _username: &str) -> Result<()> {
    bail!("not yet implemented")
}

pub fn run_key_remove(_data_dir: &PathBuf, _fingerprint: &str) -> Result<()> {
    bail!("not yet implemented")
}
