//! SSH command handling for `mgs-ssh`.
//!
//! Parses `SSH_ORIGINAL_COMMAND` (set by `sshd`), identifies the user by
//! SSH key fingerprint, and delegates to the appropriate `git-upload-pack`
//! or `git-receive-pack` process.

use anyhow::{Context, Result, bail};
use std::env;
use std::path::PathBuf;

use crate::db::Database;
use crate::git::{exec_git_receive_pack, exec_git_upload_pack, repo_disk_path, validate_repo_name};

/// Parsed Git command from `SSH_ORIGINAL_COMMAND`.
enum GitCommand {
    /// `git-upload-pack` — used by `git clone` and `git fetch`.
    UploadPack,
    /// `git-receive-pack` — used by `git push`.
    ReceivePack,
}

/// Parses `SSH_ORIGINAL_COMMAND` into a [`GitCommand`] and normalized repository name.
///
/// Expected formats:
/// - `git-upload-pack 'repo.git'`
/// - `git-receive-pack 'repo'`
///
/// Handles single/double/no quotes and strips trailing `.git` from the repo name.
fn parse_command(original: &str) -> Result<(GitCommand, String)> {
    let original = original.trim();
    let parts: Vec<&str> = original.splitn(3, ' ').collect();
    if parts.len() != 2 {
        bail!("unexpected command format: {}", original);
    }

    let cmd = parts[0];
    let mut repo_arg = parts[1].trim_matches('\'').trim_matches('"');

    if repo_arg.ends_with(".git") {
        repo_arg = &repo_arg[..repo_arg.len() - 4];
    }

    let git_cmd = match cmd {
        "git-upload-pack" => GitCommand::UploadPack,
        "git-receive-pack" => GitCommand::ReceivePack,
        _ => bail!("unsupported git command: {}", cmd),
    };

    Ok((git_cmd, repo_arg.to_string()))
}

/// Main entry point for the `mgs-ssh` binary.
///
/// Called by `sshd` via `authorized_keys` `command=` directive. The `fingerprint`
/// argument identifies the connecting user's SSH key.
///
/// Flow:
/// 1. Read `SSH_ORIGINAL_COMMAND` from environment
/// 2. Parse the git command and repository name
/// 3. Look up user by fingerprint in the database
/// 4. Execute the corresponding git pack process
pub fn handle_ssh_command(fingerprint: &str) -> Result<()> {
    let original_cmd = env::var("SSH_ORIGINAL_COMMAND").context("SSH_ORIGINAL_COMMAND not set")?;

    let (git_cmd, repo_name) = parse_command(&original_cmd)?;
    validate_repo_name(&repo_name)?;

    let data_dir = get_data_dir()?;
    let db_path = data_dir.join("mgs.db");
    let db = Database::open(&db_path)?;

    let _user = db
        .find_user_by_fingerprint(fingerprint)?
        .with_context(|| format!("no user found for key {}", fingerprint))?;

    let _repo = db
        .find_repo(&repo_name)?
        .with_context(|| format!("repository not found: {}", repo_name))?;

    let disk_path = repo_disk_path(&data_dir, &repo_name);
    if !disk_path.exists() {
        bail!("repository disk path not found: {}", disk_path.display());
    }

    match git_cmd {
        GitCommand::UploadPack => exec_git_upload_pack(&disk_path),
        GitCommand::ReceivePack => exec_git_receive_pack(&disk_path),
    }
}

/// Returns the MGS data directory.
///
/// Checks `MGS_HOME` env var first, falls back to the directory
/// containing the `mgs-ssh` binary.
fn get_data_dir() -> Result<PathBuf> {
    if let Ok(home) = env::var("MGS_HOME") {
        return Ok(PathBuf::from(home));
    }
    let exe = env::current_exe().context("failed to determine executable path")?;
    let dir = exe
        .parent()
        .context("failed to determine executable directory")?;
    Ok(dir.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_upload_pack_with_quotes() {
        let (cmd, repo) = parse_command("git-upload-pack 'myrepo.git'").unwrap();
        assert!(matches!(cmd, GitCommand::UploadPack));
        assert_eq!(repo, "myrepo");
    }

    #[test]
    fn test_parse_receive_pack_with_quotes() {
        let (cmd, repo) = parse_command("git-receive-pack 'myrepo'").unwrap();
        assert!(matches!(cmd, GitCommand::ReceivePack));
        assert_eq!(repo, "myrepo");
    }

    #[test]
    fn test_parse_strips_dot_git() {
        let (_, repo) = parse_command("git-upload-pack 'project.git'").unwrap();
        assert_eq!(repo, "project");
    }

    #[test]
    fn test_parse_no_dot_git() {
        let (_, repo) = parse_command("git-upload-pack 'project'").unwrap();
        assert_eq!(repo, "project");
    }

    #[test]
    fn test_parse_double_quotes() {
        let (cmd, repo) = parse_command("git-upload-pack \"myrepo.git\"").unwrap();
        assert!(matches!(cmd, GitCommand::UploadPack));
        assert_eq!(repo, "myrepo");
    }

    #[test]
    fn test_parse_no_quotes() {
        let (cmd, repo) = parse_command("git-upload-pack myrepo").unwrap();
        assert!(matches!(cmd, GitCommand::UploadPack));
        assert_eq!(repo, "myrepo");
    }

    #[test]
    fn test_parse_unsupported_command() {
        assert!(parse_command("git-repo-list 'repo'").is_err());
    }

    #[test]
    fn test_parse_empty() {
        assert!(parse_command("").is_err());
    }

    #[test]
    fn test_parse_too_many_parts() {
        assert!(parse_command("git-upload-pack repo extra").is_err());
    }

    #[test]
    fn test_parse_with_whitespace() {
        let (_, repo) = parse_command("  git-upload-pack 'myrepo'  ").unwrap();
        assert_eq!(repo, "myrepo");
    }
}
