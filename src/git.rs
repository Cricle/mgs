use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Validate repository name: only [a-zA-Z0-9/_.-] allowed.
pub fn validate_repo_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("repository name cannot be empty");
    }
    if name.contains("..") {
        bail!("repository name cannot contain '..'");
    }
    for ch in name.chars() {
        if !ch.is_ascii_alphanumeric() && ch != '/' && ch != '_' && ch != '.' && ch != '-' {
            bail!("invalid character '{}' in repository name", ch);
        }
    }
    if name.ends_with(".git") {
        // Allowed but we don't require it
    }
    Ok(())
}

/// Validate username: only [a-zA-Z0-9_-] allowed.
pub fn validate_username(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("username cannot be empty");
    }
    for ch in name.chars() {
        if !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-' {
            bail!("invalid character '{}' in username", ch);
        }
    }
    Ok(())
}

/// Get the disk path for a repository.
pub fn repo_disk_path(data_dir: &Path, repo_name: &str) -> PathBuf {
    let mut path = data_dir.join("repos").join(repo_name);
    if !path.to_string_lossy().ends_with(".git") {
        path = PathBuf::from(format!("{}.git", path.display()));
    }
    path
}

/// Initialize a bare git repository at the given path.
pub fn init_bare_repo(path: &Path) -> Result<()> {
    if path.exists() {
        bail!("repository already exists at {}", path.display());
    }
    let output = Command::new("git")
        .args(["init", "--bare", path.to_str().unwrap()])
        .output()
        .context("failed to run git init --bare")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git init --bare failed: {}", stderr);
    }
    Ok(())
}

/// Execute git-upload-pack (for clone/fetch) with stdin/stdout piped.
pub fn exec_git_upload_pack(repo_path: &Path) -> Result<()> {
    let status = Command::new("git-upload-pack")
        .arg(repo_path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("failed to execute git-upload-pack")?;

    if !status.success() {
        bail!("git-upload-pack exited with {}", status);
    }
    Ok(())
}

/// Execute git-receive-pack (for push) with stdin/stdout piped.
pub fn exec_git_receive_pack(repo_path: &Path) -> Result<()> {
    let status = Command::new("git-receive-pack")
        .arg(repo_path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("failed to execute git-receive-pack")?;

    if !status.success() {
        bail!("git-receive-pack exited with {}", status);
    }
    Ok(())
}
