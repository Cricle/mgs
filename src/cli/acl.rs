use anyhow::{bail, Result};
use std::path::PathBuf;

pub fn run_acl_grant(_data_dir: &PathBuf, _username: &str, _repo: &str, _perm: &str) -> Result<()> {
    bail!("not yet implemented")
}

pub fn run_acl_revoke(_data_dir: &PathBuf, _username: &str, _repo: &str) -> Result<()> {
    bail!("not yet implemented")
}

pub fn run_acl_list(_data_dir: &PathBuf, _repo: &str) -> Result<()> {
    bail!("not yet implemented")
}
