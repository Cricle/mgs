use super::*;
use std::fs;

#[test]
fn test_push_and_clone_roundtrip() {
    let home = test_home();

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
fn test_concurrent_pushes() {
    let home = test_home();

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
    git_cmd(&dir2, &wrapper, &["pull", "--no-rebase", "origin", &b2]);
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
fn test_push_non_fast_forward_rejected() {
    let home = test_home();

    let key_path = generate_key(&home, "nff_user");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &[
            "user",
            "add",
            "nff_user",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs(&home, &["repo", "create", "nffrepo", "--owner", "nff_user"]);

    let wrapper = create_ssh_wrapper(&home, &fp);

    let dir_a = home.join("nff_a");
    let dir_b = home.join("nff_b");
    fs::create_dir_all(&dir_a).unwrap();
    fs::create_dir_all(&dir_b).unwrap();

    git_cmd(&dir_a, &wrapper, &["clone", "mgs:nffrepo.git", "."]);
    git_cmd(&dir_b, &wrapper, &["clone", "mgs:nffrepo.git", "."]);
    git_config_user(&dir_a);
    git_config_user(&dir_b);

    // Push from A
    git_commit(&dir_a, "file.txt", "v1", "v1");
    let branch = current_branch(&dir_a);
    git_cmd(&dir_a, &wrapper, &["push", "origin", &branch]);

    // B also commits (diverged history)
    git_commit(&dir_b, "other.txt", "other", "other");

    // B's push should be rejected (non-fast-forward)
    let output = Command::new("git")
        .env("GIT_SSH_COMMAND", format!("'{}'", wrapper.display()))
        .current_dir(&dir_b)
        .args(["push", "origin", &branch])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("rejected")
            || stderr.contains("non-fast-forward")
            || stderr.contains("failed")
    );
}
