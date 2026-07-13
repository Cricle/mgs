//! End-to-end tests that simulate real git clone/push/pull flows
//! through the `mgs-ssh` binary.

mod clone;
mod file;
mod key;
mod pull;
mod push;
mod repo;
mod user;

mod branch;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn test_home() -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = PathBuf::from(format!("/tmp/mgs-e2e-{}", id));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn mgs(home: &Path, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_mgs"))
        .env("MGS_HOME", home.to_str().unwrap())
        .args(args)
        .output()
        .expect("failed to run mgs");
    assert!(
        output.status.success(),
        "mgs {:?} failed:\nstdout: {}\nstderr: {}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

pub fn generate_key(home: &Path, name: &str) -> PathBuf {
    let key_path = home.join(format!("{}.pub", name));
    let output = Command::new("ssh-keygen")
        .args([
            "-t",
            "ed25519",
            "-f",
            &home.join(name).to_string_lossy(),
            "-N",
            "",
            "-C",
            &format!("{}@e2e", name),
        ])
        .output()
        .expect("failed to run ssh-keygen");
    assert!(
        output.status.success(),
        "ssh-keygen failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    key_path
}

pub fn key_fingerprint(key_path: &Path) -> String {
    let output = Command::new("ssh-keygen")
        .args(["-lf", &key_path.to_string_lossy()])
        .output()
        .expect("failed to get fingerprint");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .split_whitespace()
        .find(|s| s.starts_with("SHA256:"))
        .unwrap()
        .to_string()
}

pub fn create_ssh_wrapper(home: &Path, fingerprint: &str) -> PathBuf {
    let safe_name: String = fingerprint
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let wrapper = home.join(format!("git-ssh-wrapper-{}.sh", safe_name));
    let mgs_path = PathBuf::from(env!("CARGO_BIN_EXE_mgs"));
    let mgs_ssh = mgs_path.parent().unwrap().join("mgs-ssh");
    assert!(
        mgs_ssh.exists(),
        "mgs-ssh binary not found at {}",
        mgs_ssh.display()
    );
    let content = format!(
        r#"#!/bin/sh
export SSH_ORIGINAL_COMMAND="$2"
export MGS_HOME="{}"
exec "{}" "{}"
"#,
        home.display(),
        mgs_ssh.display(),
        fingerprint
    );
    fs::write(&wrapper, content).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    }
    wrapper
}

pub fn git_cmd(home: &Path, wrapper: &Path, args: &[&str]) -> String {
    let output = git_cmd_raw(home, wrapper, args);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(
        output.status.success(),
        "git {:?} failed:\nstdout: {}\nstderr: {}",
        args,
        stdout,
        stderr
    );
    stdout + &stderr
}

pub fn git_cmd_raw(home: &Path, wrapper: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .env("GIT_SSH_COMMAND", format!("'{}'", wrapper.display()))
        .current_dir(home)
        .args(args)
        .output()
        .expect("failed to run git")
}

/// Returns the default branch name of the current git repo.
pub fn current_branch(dir: &Path) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(["branch", "--show-current"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Configures git user in a directory.
pub fn git_config_user(dir: &Path) {
    Command::new("git")
        .current_dir(dir)
        .args(["config", "user.email", "test@e2e"])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(dir)
        .args(["config", "user.name", "E2E Test"])
        .output()
        .unwrap();
}

/// Creates a commit with the given file content and message.
pub fn git_commit(dir: &Path, filename: &str, content: &str, message: &str) {
    fs::write(dir.join(filename), content).unwrap();
    Command::new("git")
        .current_dir(dir)
        .args(["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(dir)
        .args(["commit", "-m", message])
        .output()
        .unwrap();
}
