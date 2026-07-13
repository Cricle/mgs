//! Integration tests for the `mgs` CLI.

mod init;
mod repo;
mod user;
mod workflow;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn mgs_home() -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = PathBuf::from(format!("/tmp/mgs-test-{}", id));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn mgs_cmd_inner(home: &Path, args: &[&str]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_mgs"))
        .env("MGS_HOME", home.to_str().unwrap())
        .args(args)
        .output()
        .expect("failed to run mgs");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stdout, stderr)
}

pub fn mgs_cmd(home: &Path, args: &[&str]) -> String {
    let (success, stdout, stderr) = mgs_cmd_inner(home, args);
    if !success {
        panic!(
            "mgs {:?} failed:\nstdout: {}\nstderr: {}",
            args, stdout, stderr
        );
    }
    stdout
}

pub fn mgs_cmd_fails(home: &Path, args: &[&str]) -> String {
    let (success, _, stderr) = mgs_cmd_inner(home, args);
    assert!(!success, "mgs {:?} should have failed but succeeded", args);
    stderr
}

/// Generate a test SSH keypair and return the public key file path.
pub fn generate_test_key(home: &Path, name: &str) -> PathBuf {
    let key_path = home.join(format!("{}.pub", name));
    let output = Command::new("ssh-keygen")
        .args([
            "-t",
            "ed25519",
            "-f",
            home.join(name).to_str().unwrap(),
            "-N",
            "",
            "-C",
            &format!("{}@test", name),
        ])
        .output()
        .expect("failed to run ssh-keygen");
    assert!(
        output.status.success(),
        "ssh-keygen failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(key_path.exists(), "public key file not created");
    key_path
}
