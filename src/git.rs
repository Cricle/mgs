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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // --- validate_repo_name ---

    #[test]
    fn test_validate_repo_name_valid() {
        assert!(validate_repo_name("myrepo").is_ok());
        assert!(validate_repo_name("team/project").is_ok());
        assert!(validate_repo_name("my-repo_v2").is_ok());
        assert!(validate_repo_name("a.b/c-d_e").is_ok());
    }

    #[test]
    fn test_validate_repo_name_empty() {
        assert!(validate_repo_name("").is_err());
    }

    #[test]
    fn test_validate_repo_name_traversal() {
        assert!(validate_repo_name("../etc/passwd").is_err());
        assert!(validate_repo_name("foo/../bar").is_err());
        assert!(validate_repo_name("foo/..").is_err());
    }

    #[test]
    fn test_validate_repo_name_invalid_chars() {
        assert!(validate_repo_name("my repo").is_err());
        assert!(validate_repo_name("my@repo").is_err());
        assert!(validate_repo_name("repo!").is_err());
        assert!(validate_repo_name("a b").is_err());
    }

    // --- normalize_repo_name ---

    #[test]
    fn test_normalize_strips_git() {
        assert_eq!(normalize_repo_name("myrepo.git"), "myrepo");
        assert_eq!(normalize_repo_name("team/project.git"), "team/project");
    }

    #[test]
    fn test_normalize_no_git() {
        assert_eq!(normalize_repo_name("myrepo"), "myrepo");
        assert_eq!(normalize_repo_name("team/project"), "team/project");
    }

    // --- validate_username ---

    #[test]
    fn test_validate_username_valid() {
        assert!(validate_username("alice").is_ok());
        assert!(validate_username("user_1").is_ok());
        assert!(validate_username("my-user").is_ok());
        assert!(validate_username("A1_b2-C3").is_ok());
    }

    #[test]
    fn test_validate_username_empty() {
        assert!(validate_username("").is_err());
    }

    #[test]
    fn test_validate_username_invalid_chars() {
        assert!(validate_username("my user").is_err());
        assert!(validate_username("user@host").is_err());
        assert!(validate_username("a.b").is_err());
        assert!(validate_username("a/b").is_err());
    }

    // --- repo_disk_path ---

    #[test]
    fn test_repo_disk_path_appends_git() {
        let data = PathBuf::from("/data");
        let path = repo_disk_path(&data, "myrepo");
        assert_eq!(path, PathBuf::from("/data/repos/myrepo.git"));
    }

    #[test]
    fn test_repo_disk_path_nested() {
        let data = PathBuf::from("/data");
        let path = repo_disk_path(&data, "team/project");
        assert_eq!(path, PathBuf::from("/data/repos/team/project.git"));
    }

    // --- init_bare_repo ---

    #[test]
    fn test_init_bare_repo_creates_directory() {
        let tmp = TempDir::new().unwrap();
        let repo_path = tmp.path().join("test.git");
        init_bare_repo(&repo_path).unwrap();
        assert!(repo_path.exists());
        assert!(repo_path.join("HEAD").exists());
        assert!(repo_path.join("objects").exists());
    }

    #[test]
    fn test_init_bare_repo_fails_if_exists() {
        let tmp = TempDir::new().unwrap();
        let repo_path = tmp.path().join("test.git");
        init_bare_repo(&repo_path).unwrap();
        assert!(init_bare_repo(&repo_path).is_err());
    }
}
