use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn mgs_home() -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = PathBuf::from(format!("/tmp/mgs-test-{}", id));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn mgs_cmd(home: &PathBuf, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_mgs"))
        .env("MGS_HOME", home.to_str().unwrap())
        .args(args)
        .output()
        .expect("failed to run mgs");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        panic!("mgs {:?} failed:\nstdout: {}\nstderr: {}", args, stdout, stderr);
    }
    stdout
}

fn mgs_cmd_fails(home: &PathBuf, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_mgs"))
        .env("MGS_HOME", home.to_str().unwrap())
        .args(args)
        .output()
        .expect("failed to run mgs");
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(!output.status.success(), "mgs {:?} should have failed but succeeded", args);
    stderr
}

/// Generate a test SSH keypair and return the public key file path.
fn generate_test_key(home: &PathBuf, name: &str) -> PathBuf {
    let key_path = home.join(format!("{}.pub", name));
    let output = Command::new("ssh-keygen")
        .args([
            "-t", "ed25519",
            "-f", home.join(name).to_str().unwrap(),
            "-N", "",
            "-C", &format!("{}@test", name),
        ])
        .output()
        .expect("failed to run ssh-keygen");
    assert!(output.status.success(), "ssh-keygen failed: {}", String::from_utf8_lossy(&output.stderr));
    assert!(key_path.exists(), "public key file not created");
    key_path
}

// ---- Init tests ----

#[test]
fn test_init() {
    let home = mgs_home();
    let out = mgs_cmd(&home, &["init"]);
    assert!(out.contains("Initialized"));
    assert!(home.join("mgs.db").exists());
    assert!(home.join("repos").exists());
}

#[test]
fn test_init_idempotent() {
    let home = mgs_home();
    let out1 = mgs_cmd(&home, &["init"]);
    assert!(out1.contains("Initialized"));

    let out2 = mgs_cmd(&home, &["init"]);
    assert!(out2.contains("already initialized"));
}

// ---- User tests ----

#[test]
fn test_user_add_and_list() {
    let home = mgs_home();
    mgs_cmd(&home, &["init"]);

    let key_path = generate_test_key(&home, "test_key");
    let out = mgs_cmd(&home, &["user", "add", "alice", "--key", key_path.to_str().unwrap()]);
    assert!(out.contains("Created user 'alice'"));
    assert!(out.contains("SHA256:"));

    let list_out = mgs_cmd(&home, &["user", "list"]);
    assert!(list_out.contains("alice"));
    assert!(list_out.contains("1 keys"));
}

#[test]
fn test_user_remove() {
    let home = mgs_home();
    mgs_cmd(&home, &["init"]);

    let key_path = generate_test_key(&home, "rm_key");
    mgs_cmd(&home, &["user", "add", "bob", "--key", key_path.to_str().unwrap()]);

    let out = mgs_cmd(&home, &["user", "remove", "bob"]);
    assert!(out.contains("Removed user 'bob'"));

    let list_out = mgs_cmd(&home, &["user", "list"]);
    assert!(list_out.contains("No users found"));
}

#[test]
fn test_user_add_duplicate() {
    let home = mgs_home();
    mgs_cmd(&home, &["init"]);

    let key_path = generate_test_key(&home, "dup_key");
    mgs_cmd(&home, &["user", "add", "charlie", "--key", key_path.to_str().unwrap()]);

    let err = mgs_cmd_fails(&home, &["user", "add", "charlie", "--key", key_path.to_str().unwrap()]);
    assert!(err.contains("already exists"));
}

// ---- Repo tests ----

#[test]
fn test_repo_create_and_list() {
    let home = mgs_home();
    mgs_cmd(&home, &["init"]);

    let key_path = generate_test_key(&home, "repo_key");
    mgs_cmd(&home, &["user", "add", "owner1", "--key", key_path.to_str().unwrap()]);

    let out = mgs_cmd(&home, &["repo", "create", "team/project", "--owner", "owner1"]);
    assert!(out.contains("Created repository 'team/project'"));
    assert!(out.contains("owner: owner1"));

    // Verify the bare repo exists on disk
    assert!(home.join("repos/team/project.git").exists());

    let list_out = mgs_cmd(&home, &["repo", "list"]);
    assert!(list_out.contains("team/project"));
    assert!(list_out.contains("owner: owner1"));
}

#[test]
fn test_repo_remove() {
    let home = mgs_home();
    mgs_cmd(&home, &["init"]);

    let key_path = generate_test_key(&home, "rr_key");
    mgs_cmd(&home, &["user", "add", "owner2", "--key", key_path.to_str().unwrap()]);
    mgs_cmd(&home, &["repo", "create", "my/repo", "--owner", "owner2"]);

    let out = mgs_cmd(&home, &["repo", "remove", "my/repo"]);
    assert!(out.contains("Removed repository 'my/repo'"));

    let list_out = mgs_cmd(&home, &["repo", "list"]);
    assert!(list_out.contains("No repositories found"));
}

// ---- ACL tests ----

#[test]
fn test_acl_grant_and_list() {
    let home = mgs_home();
    mgs_cmd(&home, &["init"]);

    let key_path = generate_test_key(&home, "acl_key");
    mgs_cmd(&home, &["user", "add", "dev1", "--key", key_path.to_str().unwrap()]);

    let key_path2 = generate_test_key(&home, "acl_key2");
    mgs_cmd(&home, &["user", "add", "dev2", "--key", key_path2.to_str().unwrap()]);

    mgs_cmd(&home, &["repo", "create", "org/app", "--owner", "dev1"]);

    let out = mgs_cmd(&home, &["acl", "grant", "dev2", "org/app", "--perm", "write"]);
    assert!(out.contains("Granted write"));
    assert!(out.contains("dev2"));
    assert!(out.contains("org/app"));

    let list_out = mgs_cmd(&home, &["acl", "list", "org/app"]);
    assert!(list_out.contains("Repository: org/app"));
    assert!(list_out.contains("owner: dev1"));
    assert!(list_out.contains("dev2"));
    assert!(list_out.contains("write"));
}

#[test]
fn test_acl_revoke() {
    let home = mgs_home();
    mgs_cmd(&home, &["init"]);

    let key_path = generate_test_key(&home, "rev_key");
    mgs_cmd(&home, &["user", "add", "dev3", "--key", key_path.to_str().unwrap()]);

    let key_path2 = generate_test_key(&home, "rev_key2");
    mgs_cmd(&home, &["user", "add", "dev4", "--key", key_path2.to_str().unwrap()]);

    mgs_cmd(&home, &["repo", "create", "org/svc", "--owner", "dev3"]);
    mgs_cmd(&home, &["acl", "grant", "dev4", "org/svc", "--perm", "read"]);

    let out = mgs_cmd(&home, &["acl", "revoke", "dev4", "org/svc"]);
    assert!(out.contains("Revoked permissions"));

    let list_out = mgs_cmd(&home, &["acl", "list", "org/svc"]);
    assert!(list_out.contains("No additional permissions granted"));
}

// ---- Full end-to-end workflow ----

#[test]
fn test_full_workflow() {
    let home = mgs_home();
    mgs_cmd(&home, &["init"]);

    // Create users
    let key1 = generate_test_key(&home, "admin_key");
    mgs_cmd(&home, &["user", "add", "admin", "--key", key1.to_str().unwrap()]);

    let key2 = generate_test_key(&home, "dev_key");
    mgs_cmd(&home, &["user", "add", "developer", "--key", key2.to_str().unwrap()]);

    // Create repo
    let out = mgs_cmd(&home, &["repo", "create", "team/backend", "--owner", "admin"]);
    assert!(out.contains("Created repository 'team/backend'"));

    // Grant write to developer
    mgs_cmd(&home, &["acl", "grant", "developer", "team/backend", "--perm", "write"]);

    // Verify user list
    let users = mgs_cmd(&home, &["user", "list"]);
    assert!(users.contains("admin"));
    assert!(users.contains("developer"));

    // Verify repo list
    let repos = mgs_cmd(&home, &["repo", "list"]);
    assert!(repos.contains("team/backend"));
    assert!(repos.contains("owner: admin"));

    // Verify ACL
    let acl = mgs_cmd(&home, &["acl", "list", "team/backend"]);
    assert!(acl.contains("developer"));
    assert!(acl.contains("write"));

    // Revoke and verify
    mgs_cmd(&home, &["acl", "revoke", "developer", "team/backend"]);
    let acl_after = mgs_cmd(&home, &["acl", "list", "team/backend"]);
    assert!(acl_after.contains("No additional permissions granted"));

    // Remove user and verify
    mgs_cmd(&home, &["user", "remove", "developer"]);
    let users_after = mgs_cmd(&home, &["user", "list"]);
    assert!(!users_after.contains("developer"));
    assert!(users_after.contains("admin"));
}

// ---- Error cases ----

#[test]
fn test_repo_create_nonexistent_owner() {
    let home = mgs_home();
    mgs_cmd(&home, &["init"]);

    let err = mgs_cmd_fails(&home, &["repo", "create", "x/y", "--owner", "ghost"]);
    assert!(err.contains("not found"));
}

#[test]
fn test_acl_grant_invalid_perm() {
    let home = mgs_home();
    mgs_cmd(&home, &["init"]);

    let key_path = generate_test_key(&home, "badperm_key");
    mgs_cmd(&home, &["user", "add", "u1", "--key", key_path.to_str().unwrap()]);
    mgs_cmd(&home, &["repo", "create", "r1", "--owner", "u1"]);

    let err = mgs_cmd_fails(&home, &["acl", "grant", "u1", "r1", "--perm", "superadmin"]);
    assert!(err.contains("invalid permission"));
}

#[test]
fn test_user_add_nonexistent_key_file() {
    let home = mgs_home();
    mgs_cmd(&home, &["init"]);

    let err = mgs_cmd_fails(&home, &["user", "add", "u1", "--key", "/tmp/nonexistent-key.pub"]);
    assert!(err.contains("failed to read key file"));
}
