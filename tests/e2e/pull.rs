use super::*;
use std::fs;

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
fn test_pull_with_merge() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "puller");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &["user", "add", "puller", "--key", key_path.to_str().unwrap()],
    );
    mgs(&home, &["repo", "create", "pullrepo", "--owner", "puller"]);

    let wrapper = create_ssh_wrapper(&home, &fp);

    let dir_a = home.join("pull_a");
    let dir_b = home.join("pull_b");
    fs::create_dir_all(&dir_a).unwrap();
    fs::create_dir_all(&dir_b).unwrap();

    git_cmd(&dir_a, &wrapper, &["clone", "mgs:pullrepo.git", "."]);
    git_cmd(&dir_b, &wrapper, &["clone", "mgs:pullrepo.git", "."]);
    git_config_user(&dir_a);
    git_config_user(&dir_b);

    // Push from A
    git_commit(&dir_a, "a_file.txt", "from a", "a commit");
    let branch = current_branch(&dir_a);
    git_cmd(&dir_a, &wrapper, &["push", "origin", &branch]);

    // B commits, pulls A's changes (merge), then pushes
    git_commit(&dir_b, "b_file.txt", "from b", "b commit");
    git_cmd(
        &dir_b,
        &wrapper,
        &[
            "pull",
            "--no-rebase",
            "--allow-unrelated-histories",
            "origin",
            &branch,
        ],
    );
    git_cmd(&dir_b, &wrapper, &["push", "origin", &branch]);

    // Pull in A — should get B's file
    git_cmd(
        &dir_a,
        &wrapper,
        &[
            "pull",
            "--no-rebase",
            "--allow-unrelated-histories",
            "origin",
            &branch,
        ],
    );
    assert!(dir_a.join("a_file.txt").exists());
    assert!(dir_a.join("b_file.txt").exists());
}

#[test]
fn test_fetch_all_branches() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "fetchall_user");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &[
            "user",
            "add",
            "fetchall_user",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs(
        &home,
        &["repo", "create", "fetchallrepo", "--owner", "fetchall_user"],
    );

    let wrapper = create_ssh_wrapper(&home, &fp);
    let work = home.join("fetchall_work");
    fs::create_dir_all(&work).unwrap();

    git_cmd(&work, &wrapper, &["clone", "mgs:fetchallrepo.git", "."]);
    git_config_user(&work);

    // Push initial commit on default branch
    git_commit(&work, "main.txt", "main", "init");
    let branch = current_branch(&work);
    git_cmd(&work, &wrapper, &["push", "origin", &branch]);

    // Create and push multiple branches
    for name in &["feature-a", "feature-b", "hotfix-x"] {
        Command::new("git")
            .current_dir(&work)
            .args(["checkout", "-b", name])
            .output()
            .unwrap();
        git_commit(
            &work,
            &format!("{}.txt", name),
            name,
            &format!("work on {}", name),
        );
        git_cmd(&work, &wrapper, &["push", "origin", name]);
        Command::new("git")
            .current_dir(&work)
            .args(["checkout", &branch])
            .output()
            .unwrap();
    }

    // Fresh clone and fetch all
    let verify = home.join("fetchall_verify");
    fs::create_dir_all(&verify).unwrap();
    git_cmd(&verify, &wrapper, &["clone", "mgs:fetchallrepo.git", "."]);
    git_cmd(&verify, &wrapper, &["fetch", "--all"]);

    let branches = Command::new("git")
        .current_dir(&verify)
        .args(["branch", "-r"])
        .output()
        .unwrap();
    let branches_str = String::from_utf8_lossy(&branches.stdout);
    assert!(branches_str.contains("origin/feature-a"));
    assert!(branches_str.contains("origin/feature-b"));
    assert!(branches_str.contains("origin/hotfix-x"));
}
