use super::*;
use std::fs;

#[test]
fn test_branch_operations() {
    let home = test_home();

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
