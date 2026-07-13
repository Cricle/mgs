use super::*;
use std::fs;

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
fn test_multiple_users_same_repo() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_a = generate_key(&home, "user_a");
    let fp_a = key_fingerprint(&key_a);
    mgs(
        &home,
        &["user", "add", "alice", "--key", key_a.to_str().unwrap()],
    );

    let key_b = generate_key(&home, "user_b");
    let fp_b = key_fingerprint(&key_b);
    mgs(
        &home,
        &["user", "add", "bob", "--key", key_b.to_str().unwrap()],
    );

    mgs(&home, &["repo", "create", "shared", "--owner", "alice"]);

    let wrapper_a = create_ssh_wrapper(&home, &fp_a);
    let wrapper_b = create_ssh_wrapper(&home, &fp_b);

    // Alice clones and pushes
    let dir_a = home.join("alice_work");
    fs::create_dir_all(&dir_a).unwrap();
    git_cmd(&dir_a, &wrapper_a, &["clone", "mgs:shared.git", "."]);
    git_config_user(&dir_a);
    git_commit(&dir_a, "alice.txt", "from alice", "alice work");
    let branch = current_branch(&dir_a);
    git_cmd(&dir_a, &wrapper_a, &["push", "origin", &branch]);

    // Bob clones, sees alice's work, pushes his own
    let dir_b = home.join("bob_work");
    fs::create_dir_all(&dir_b).unwrap();
    git_cmd(&dir_b, &wrapper_b, &["clone", "mgs:shared.git", "."]);
    git_config_user(&dir_b);
    assert!(dir_b.join("alice.txt").exists());
    git_commit(&dir_b, "bob.txt", "from bob", "bob work");
    git_cmd(&dir_b, &wrapper_b, &["push", "origin", &branch]);

    // Verify both files
    let verify = home.join("shared_verify");
    fs::create_dir_all(&verify).unwrap();
    git_cmd(&verify, &wrapper_a, &["clone", "mgs:shared.git", "."]);
    assert!(verify.join("alice.txt").exists());
    assert!(verify.join("bob.txt").exists());
}

#[test]
fn test_multiple_keys_per_user() {
    let home = test_home();
    mgs(&home, &["init"]);

    // Create user with first key
    let key1 = generate_key(&home, "key1");
    let fp1 = key_fingerprint(&key1);
    mgs(
        &home,
        &["user", "add", "multikey", "--key", key1.to_str().unwrap()],
    );

    // Add second key
    let key2 = generate_key(&home, "key2");
    let fp2 = key_fingerprint(&key2);
    mgs(
        &home,
        &[
            "user",
            "key",
            "add",
            "multikey",
            "--key",
            key2.to_str().unwrap(),
        ],
    );

    mgs(&home, &["repo", "create", "mkrepo", "--owner", "multikey"]);

    // Push with first key
    let wrapper1 = create_ssh_wrapper(&home, &fp1);
    let dir1 = home.join("key1_work");
    fs::create_dir_all(&dir1).unwrap();
    git_cmd(&dir1, &wrapper1, &["clone", "mgs:mkrepo.git", "."]);
    git_config_user(&dir1);
    git_commit(&dir1, "from_key1.txt", "key1", "from key1");
    let branch = current_branch(&dir1);
    git_cmd(&dir1, &wrapper1, &["push", "origin", &branch]);

    // Clone with second key — should work
    let wrapper2 = create_ssh_wrapper(&home, &fp2);
    let dir2 = home.join("key2_work");
    fs::create_dir_all(&dir2).unwrap();
    git_cmd(&dir2, &wrapper2, &["clone", "mgs:mkrepo.git", "."]);
    assert!(dir2.join("from_key1.txt").exists());

    // Push with second key
    git_config_user(&dir2);
    git_commit(&dir2, "from_key2.txt", "key2", "from key2");
    git_cmd(&dir2, &wrapper2, &["push", "origin", &branch]);
}
