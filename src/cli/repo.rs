use anyhow::{bail, Result};
use std::path::PathBuf;

pub fn run_repo_create(_data_dir: &PathBuf, _name: &str, _owner: Option<&str>) -> Result<()> {
    bail!("not yet implemented")
}

pub fn run_repo_list(_data_dir: &PathBuf) -> Result<()> {
    bail!("not yet implemented")
}

pub fn run_repo_remove(_data_dir: &PathBuf, _name: &str) -> Result<()> {
    bail!("not yet implemented")
}
