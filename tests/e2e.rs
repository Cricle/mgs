//! End-to-end tests that simulate real git clone/push/pull flows
//! through the `mgs-ssh` binary.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn test_home() -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = PathBuf::from(format!("/tmp/mgs-e2e-{}", id));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn mgs(home: &Path, args: &[&str]) -> String {
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

fn generate_key(home: &Path, name: &str) -> PathBuf {
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

fn key_fingerprint(key_path: &Path) -> String {
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

fn create_ssh_wrapper(home: &Path, fingerprint: &str) -> PathBuf {
    let wrapper = home.join("git-ssh-wrapper.sh");
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

fn git_cmd(home: &Path, wrapper: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .env("GIT_SSH_COMMAND", format!("'{}'", wrapper.display()))
        .current_dir(home)
        .args(args)
        .output()
        .expect("failed to run git");
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

/// Returns the default branch name of the current git repo.
fn current_branch(dir: &Path) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(["branch", "--show-current"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Configures git user in a directory.
fn git_config_user(dir: &Path) {
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
fn git_commit(dir: &Path, filename: &str, content: &str, message: &str) {
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

// ---- E2E Tests ----

#[test]
fn test_clone_empty_repo() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "user1");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &["user", "add", "user1", "--key", key_path.to_str().unwrap()],
    );
    mgs(&home, &["repo", "create", "myrepo", "--owner", "user1"]);

    let wrapper = create_ssh_wrapper(&home, &fp);
    let clone_dir = home.join("clone");
    fs::create_dir_all(&clone_dir).unwrap();

    let out = git_cmd(&clone_dir, &wrapper, &["clone", "mgs:myrepo.git", "."]);
    assert!(
        out.contains("Cloning into")
            || out.contains("warning: You appear to have cloned an empty repository")
    );
    assert!(clone_dir.join(".git").exists());
}

#[test]
fn test_push_and_clone_roundtrip() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "pusher");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &["user", "add", "pusher", "--key", key_path.to_str().unwrap()],
    );
    mgs(&home, &["repo", "create", "project", "--owner", "pusher"]);

    let wrapper = create_ssh_wrapper(&home, &fp);
    let push_dir = home.join("push_dir");
    fs::create_dir_all(&push_dir).unwrap();

    git_cmd(&push_dir, &wrapper, &["clone", "mgs:project.git", "."]);
    git_config_user(&push_dir);
    git_commit(&push_dir, "hello.txt", "Hello, MGS!", "initial commit");

    let branch = current_branch(&push_dir);
    git_cmd(&push_dir, &wrapper, &["push", "origin", &branch]);

    // Clone from a different directory
    let clone_dir = home.join("clone_dir");
    fs::create_dir_all(&clone_dir).unwrap();
    git_cmd(&clone_dir, &wrapper, &["clone", "mgs:project.git", "."]);

    let content = fs::read_to_string(clone_dir.join("hello.txt")).unwrap();
    assert_eq!(content, "Hello, MGS!");
}

#[test]
fn test_push_multiple_commits_and_clone() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "dev");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &["user", "add", "dev", "--key", key_path.to_str().unwrap()],
    );
    mgs(&home, &["repo", "create", "multi", "--owner", "dev"]);

    let wrapper = create_ssh_wrapper(&home, &fp);
    let work = home.join("work");
    fs::create_dir_all(&work).unwrap();

    git_cmd(&work, &wrapper, &["clone", "mgs:multi.git", "."]);
    git_config_user(&work);

    git_commit(&work, "a.txt", "file a", "add a");
    git_commit(&work, "b.txt", "file b", "add b");
    git_commit(&work, "a.txt", "file a modified", "modify a");

    let branch = current_branch(&work);
    git_cmd(&work, &wrapper, &["push", "origin", &branch]);

    // Clone and verify final state
    let verify = home.join("verify");
    fs::create_dir_all(&verify).unwrap();
    git_cmd(&verify, &wrapper, &["clone", "mgs:multi.git", "."]);

    assert_eq!(
        fs::read_to_string(verify.join("a.txt")).unwrap(),
        "file a modified"
    );
    assert_eq!(fs::read_to_string(verify.join("b.txt")).unwrap(), "file b");

    let log = Command::new("git")
        .current_dir(&verify)
        .args(["log", "--oneline"])
        .output()
        .unwrap();
    let log_str = String::from_utf8_lossy(&log.stdout);
    assert!(log_str.contains("modify a"));
    assert!(log_str.contains("add b"));
    assert!(log_str.contains("add a"));
}

#[test]
fn test_fetch_after_push() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "fetcher");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &[
            "user",
            "add",
            "fetcher",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs(
        &home,
        &["repo", "create", "fetchrepo", "--owner", "fetcher"],
    );

    let wrapper = create_ssh_wrapper(&home, &fp);

    // Clone into dir A
    let dir_a = home.join("dir_a");
    fs::create_dir_all(&dir_a).unwrap();
    git_cmd(&dir_a, &wrapper, &["clone", "mgs:fetchrepo.git", "."]);
    git_config_user(&dir_a);

    // Push a commit from A
    git_commit(&dir_a, "from_a.txt", "hello from A", "from A");
    let branch_a = current_branch(&dir_a);
    git_cmd(&dir_a, &wrapper, &["push", "origin", &branch_a]);

    // Clone into dir B (gets the commit)
    let dir_b = home.join("dir_b");
    fs::create_dir_all(&dir_b).unwrap();
    git_cmd(&dir_b, &wrapper, &["clone", "mgs:fetchrepo.git", "."]);
    git_config_user(&dir_b);

    // Push a new commit from B
    git_commit(&dir_b, "from_b.txt", "hello from B", "from B");
    let branch_b = current_branch(&dir_b);
    git_cmd(&dir_b, &wrapper, &["push", "origin", &branch_b]);

    // Fetch in A and verify it sees both commits
    git_cmd(&dir_a, &wrapper, &["fetch"]);
    let log = Command::new("git")
        .current_dir(&dir_a)
        .args(["log", "--oneline", &format!("origin/{}", branch_a)])
        .output()
        .unwrap();
    let log_str = String::from_utf8_lossy(&log.stdout);
    assert!(log_str.contains("from A"));
    assert!(log_str.contains("from B"));

    // Merge and verify
    Command::new("git")
        .current_dir(&dir_a)
        .args(["merge", &format!("origin/{}", branch_a)])
        .output()
        .unwrap();
    assert!(dir_a.join("from_b.txt").exists());
}

#[test]
fn test_reject_unauthenticated_user() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "known");
    mgs(
        &home,
        &["user", "add", "known", "--key", key_path.to_str().unwrap()],
    );
    mgs(&home, &["repo", "create", "secret", "--owner", "known"]);

    let wrapper = create_ssh_wrapper(&home, "SHA256:unknown_key_fingerprint");
    let clone_dir = home.join("bad_clone");
    fs::create_dir_all(&clone_dir).unwrap();

    let output = Command::new("git")
        .env("GIT_SSH_COMMAND", format!("'{}'", wrapper.display()))
        .current_dir(&clone_dir)
        .args(["clone", "mgs:secret.git", "."])
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn test_reject_nonexistent_repo() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "user2");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &["user", "add", "user2", "--key", key_path.to_str().unwrap()],
    );

    let wrapper = create_ssh_wrapper(&home, &fp);
    let clone_dir = home.join("no_repo");
    fs::create_dir_all(&clone_dir).unwrap();

    let output = Command::new("git")
        .env("GIT_SSH_COMMAND", format!("'{}'", wrapper.display()))
        .current_dir(&clone_dir)
        .args(["clone", "mgs:nonexistent.git", "."])
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn test_reject_invalid_repo_name() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "user3");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &["user", "add", "user3", "--key", key_path.to_str().unwrap()],
    );

    let wrapper = create_ssh_wrapper(&home, &fp);
    let clone_dir = home.join("bad_name");
    fs::create_dir_all(&clone_dir).unwrap();

    let output = Command::new("git")
        .env("GIT_SSH_COMMAND", format!("'{}'", wrapper.display()))
        .current_dir(&clone_dir)
        .args(["clone", "mgs:../etc/passwd.git", "."])
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn test_branch_operations() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "brancher");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &[
            "user",
            "add",
            "brancher",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs(
        &home,
        &["repo", "create", "branchrepo", "--owner", "brancher"],
    );

    let wrapper = create_ssh_wrapper(&home, &fp);
    let work = home.join("branch_work");
    fs::create_dir_all(&work).unwrap();

    git_cmd(&work, &wrapper, &["clone", "mgs:branchrepo.git", "."]);
    git_config_user(&work);

    // Initial commit
    git_commit(&work, "main.txt", "main branch", "main init");
    let branch = current_branch(&work);
    git_cmd(&work, &wrapper, &["push", "origin", &branch]);

    // Create feature branch
    Command::new("git")
        .current_dir(&work)
        .args(["checkout", "-b", "feature"])
        .output()
        .unwrap();
    git_commit(&work, "feature.txt", "feature branch", "feature work");
    git_cmd(&work, &wrapper, &["push", "origin", "feature"]);

    // Clone and verify both branches exist
    let verify = home.join("branch_verify");
    fs::create_dir_all(&verify).unwrap();
    git_cmd(&verify, &wrapper, &["clone", "mgs:branchrepo.git", "."]);

    let branches = Command::new("git")
        .current_dir(&verify)
        .args(["branch", "-r"])
        .output()
        .unwrap();
    let branches_str = String::from_utf8_lossy(&branches.stdout);
    assert!(branches_str.contains("origin/main") || branches_str.contains("origin/master"));
    assert!(branches_str.contains("origin/feature"));

    // Checkout feature and verify file
    Command::new("git")
        .current_dir(&verify)
        .args(["checkout", "feature"])
        .output()
        .unwrap();
    assert_eq!(
        fs::read_to_string(verify.join("feature.txt")).unwrap(),
        "feature branch"
    );
}

#[test]
fn test_tag_operations() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "tagger");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &["user", "add", "tagger", "--key", key_path.to_str().unwrap()],
    );
    mgs(&home, &["repo", "create", "tagrepo", "--owner", "tagger"]);

    let wrapper = create_ssh_wrapper(&home, &fp);
    let work = home.join("tag_work");
    fs::create_dir_all(&work).unwrap();

    git_cmd(&work, &wrapper, &["clone", "mgs:tagrepo.git", "."]);
    git_config_user(&work);

    // Commit and tag
    git_commit(&work, "v1.txt", "version 1", "v1");
    Command::new("git")
        .current_dir(&work)
        .args(["tag", "-a", "v1.0.0", "-m", "release 1.0.0"])
        .output()
        .unwrap();
    let branch = current_branch(&work);
    git_cmd(&work, &wrapper, &["push", "origin", &branch, "--tags"]);

    // Clone and verify tag
    let verify = home.join("tag_verify");
    fs::create_dir_all(&verify).unwrap();
    git_cmd(&verify, &wrapper, &["clone", "mgs:tagrepo.git", "."]);

    let tags = Command::new("git")
        .current_dir(&verify)
        .args(["tag", "-l"])
        .output()
        .unwrap();
    let tags_str = String::from_utf8_lossy(&tags.stdout);
    assert!(tags_str.contains("v1.0.0"));

    let show = Command::new("git")
        .current_dir(&verify)
        .args(["show", "v1.0.0", "--format=%s", "--no-patch"])
        .output()
        .unwrap();
    let show_str = String::from_utf8_lossy(&show.stdout);
    assert!(show_str.contains("v1"));
}

#[test]
fn test_concurrent_pushes() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "concurrent");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &[
            "user",
            "add",
            "concurrent",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs(
        &home,
        &["repo", "create", "conrepo", "--owner", "concurrent"],
    );

    let wrapper = create_ssh_wrapper(&home, &fp);

    let dir1 = home.join("dir1");
    let dir2 = home.join("dir2");
    fs::create_dir_all(&dir1).unwrap();
    fs::create_dir_all(&dir2).unwrap();

    git_cmd(&dir1, &wrapper, &["clone", "mgs:conrepo.git", "."]);
    git_cmd(&dir2, &wrapper, &["clone", "mgs:conrepo.git", "."]);

    git_config_user(&dir1);
    git_config_user(&dir2);

    // Push from dir1
    git_commit(&dir1, "from_dir1.txt", "dir1", "dir1");
    let b1 = current_branch(&dir1);
    git_cmd(&dir1, &wrapper, &["push", "origin", &b1]);

    // dir2 must pull before pushing
    let b2 = current_branch(&dir2);
    git_cmd(&dir2, &wrapper, &["pull", "origin", &b2]);
    git_commit(&dir2, "from_dir2.txt", "dir2", "dir2");
    git_cmd(&dir2, &wrapper, &["push", "origin", &b2]);

    // Verify both files exist in final state
    let final_clone = home.join("final");
    fs::create_dir_all(&final_clone).unwrap();
    git_cmd(&final_clone, &wrapper, &["clone", "mgs:conrepo.git", "."]);

    assert!(final_clone.join("from_dir1.txt").exists());
    assert!(final_clone.join("from_dir2.txt").exists());
}

#[test]
fn test_file_modification_roundtrip() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "modifier");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &[
            "user",
            "add",
            "modifier",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs(&home, &["repo", "create", "modrepo", "--owner", "modifier"]);

    let wrapper = create_ssh_wrapper(&home, &fp);
    let work = home.join("mod_work");
    fs::create_dir_all(&work).unwrap();

    git_cmd(&work, &wrapper, &["clone", "mgs:modrepo.git", "."]);
    git_config_user(&work);

    // Create file, push
    git_commit(&work, "data.txt", "version 1", "v1");
    let branch = current_branch(&work);
    git_cmd(&work, &wrapper, &["push", "origin", &branch]);

    // Modify file, push
    git_commit(&work, "data.txt", "version 2", "v2");
    git_cmd(&work, &wrapper, &["push", "origin", &branch]);

    // Delete file, push
    fs::remove_file(work.join("data.txt")).unwrap();
    Command::new("git")
        .current_dir(&work)
        .args(["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(&work)
        .args(["commit", "-m", "delete"])
        .output()
        .unwrap();
    git_cmd(&work, &wrapper, &["push", "origin", &branch]);

    // Clone and verify
    let verify = home.join("mod_verify");
    fs::create_dir_all(&verify).unwrap();
    git_cmd(&verify, &wrapper, &["clone", "mgs:modrepo.git", "."]);

    assert!(!verify.join("data.txt").exists());

    let log = Command::new("git")
        .current_dir(&verify)
        .args(["log", "--oneline"])
        .output()
        .unwrap();
    let log_str = String::from_utf8_lossy(&log.stdout);
    assert!(log_str.contains("delete"));
    assert!(log_str.contains("v2"));
    assert!(log_str.contains("v1"));
}

#[test]
fn test_binary_file_roundtrip() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "binary_user");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &[
            "user",
            "add",
            "binary_user",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs(
        &home,
        &["repo", "create", "binrepo", "--owner", "binary_user"],
    );

    let wrapper = create_ssh_wrapper(&home, &fp);
    let work = home.join("bin_work");
    fs::create_dir_all(&work).unwrap();

    git_cmd(&work, &wrapper, &["clone", "mgs:binrepo.git", "."]);
    git_config_user(&work);

    // Create binary file (random bytes)
    let binary_data: Vec<u8> = (0..256).map(|i| i as u8).collect();
    fs::write(work.join("data.bin"), &binary_data).unwrap();

    Command::new("git")
        .current_dir(&work)
        .args(["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(&work)
        .args(["commit", "-m", "binary"])
        .output()
        .unwrap();
    let branch = current_branch(&work);
    git_cmd(&work, &wrapper, &["push", "origin", &branch]);

    // Clone and verify binary content matches
    let verify = home.join("bin_verify");
    fs::create_dir_all(&verify).unwrap();
    git_cmd(&verify, &wrapper, &["clone", "mgs:binrepo.git", "."]);

    let cloned_data = fs::read(verify.join("data.bin")).unwrap();
    assert_eq!(cloned_data, binary_data);
}
