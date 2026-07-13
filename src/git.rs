//! Git repository operations.
//!
//! Provides validation, bare repo initialization, and execution of
//! `git-upload-pack` / `git-receive-pack` for SSH-based Git transport.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Validates a repository name.
///
/// Allowed characters: `[a-zA-Z0-9/_.-]`. Rejects names containing `..`
/// (path traversal) or empty strings.
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
    Ok(())
}

/// Normalizes a repository name by stripping any trailing `.git` suffix.
///
/// This ensures consistent storage regardless of whether the user includes `.git`:
/// - `"team/project.git"` → `"team/project"`
/// - `"team/project"` → `"team/project"`
pub fn normalize_repo_name(name: &str) -> &str {
    name.strip_suffix(".git").unwrap_or(name)
}

/// Validates a username.
///
/// Allowed characters: `[a-zA-Z0-9_-]`. Rejects empty strings.
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

/// Returns the on-disk path for a bare repository.
///
/// Constructs `<data_dir>/repos/<repo_name>.git`, appending `.git` if not present.
pub fn repo_disk_path(data_dir: &Path, repo_name: &str) -> PathBuf {
    let mut path = data_dir.join("repos").join(repo_name);
    if !path.to_string_lossy().ends_with(".git") {
        path = PathBuf::from(format!("{}.git", path.display()));
    }
    path
}

/// Initializes a bare Git repository at `path` via `git init --bare`.
///
/// Fails if the path already exists.
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

/// Checks that required git commands (`git`, `git-upload-pack`, `git-receive-pack`)
/// are available on the system. Returns an error with the missing command name.
pub fn check_git_commands() -> Result<()> {
    for cmd in &["git", "git-upload-pack", "git-receive-pack"] {
        // Use `which`/`command -v` to check existence, since git-upload-pack
        // and git-receive-pack don't support --version.
        let shell_cmd = if cfg!(target_os = "windows") {
            format!("where {}", cmd)
        } else {
            format!("command -v {}", cmd)
        };
        let output = Command::new("sh").args(["-c", &shell_cmd]).output();
        match output {
            Ok(o) if o.status.success() => {}
            _ => bail!("required command '{}' not found in PATH", cmd),
        }
    }
    Ok(())
}

/// Executes `git-upload-pack` for clone/fetch operations.
///
/// Inherits stdin/stdout/stderr from the parent process (the SSH session),
/// allowing the Git client to communicate directly with the pack process.
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

/// Executes `git-receive-pack` for push operations.
///
/// Inherits stdin/stdout/stderr from the parent process (the SSH session),
/// allowing the Git client to communicate directly with the pack process.
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
