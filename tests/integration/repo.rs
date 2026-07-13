use super::*;

#[test]
fn test_repo_create_and_list() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "repo_key");
    mgs_cmd(
        &home,
        &["user", "add", "owner1", "--key", key_path.to_str().unwrap()],
    );

    let out = mgs_cmd(
        &home,
        &["repo", "create", "team/project", "--owner", "owner1"],
    );
    assert!(out.contains("Created repository 'team/project'"));
    assert!(out.contains("owner: owner1"));

    // Verify the bare repo exists on disk
    assert!(home.join("repos/team/project.git").exists());
    assert!(home.join("repos/team/project.git/HEAD").exists());

    let list_out = mgs_cmd(&home, &["repo", "list"]);
    assert!(list_out.contains("team/project"));
    assert!(list_out.contains("owner: owner1"));
}

#[test]
fn test_repo_remove() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "rr_key");
    mgs_cmd(
        &home,
        &["user", "add", "owner2", "--key", key_path.to_str().unwrap()],
    );
    mgs_cmd(&home, &["repo", "create", "my/repo", "--owner", "owner2"]);

    let out = mgs_cmd(&home, &["repo", "remove", "my/repo"]);
    assert!(out.contains("Removed repository 'my/repo'"));
    assert!(!home.join("repos/my/repo.git").exists());

    let list_out = mgs_cmd(&home, &["repo", "list"]);
    assert!(list_out.contains("No repositories found"));
}

#[test]
fn test_repo_create_nonexistent_owner() {
    let home = mgs_home();
    let err = mgs_cmd_fails(&home, &["repo", "create", "x/y", "--owner", "ghost"]);
    assert!(err.contains("not found"));
}

#[test]
fn test_repo_create_duplicate() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "dup_repo_key");
    mgs_cmd(
        &home,
        &[
            "user",
            "add",
            "dupowner",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs_cmd(&home, &["repo", "create", "duprepo", "--owner", "dupowner"]);
    let err = mgs_cmd_fails(&home, &["repo", "create", "duprepo", "--owner", "dupowner"]);
    assert!(err.contains("already exists"));
}

#[test]
fn test_repo_create_invalid_name() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "inv_key");
    mgs_cmd(
        &home,
        &[
            "user",
            "add",
            "invowner",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    let err = mgs_cmd_fails(
        &home,
        &["repo", "create", "../etc/passwd", "--owner", "invowner"],
    );
    assert!(err.contains("cannot contain '..'"));
}

#[test]
fn test_repo_create_with_dot_git_suffix() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "suffix_key");
    mgs_cmd(
        &home,
        &[
            "user",
            "add",
            "suffixowner",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    // Create with .git suffix — should be stripped
    mgs_cmd(
        &home,
        &["repo", "create", "myproject.git", "--owner", "suffixowner"],
    );

    let list_out = mgs_cmd(&home, &["repo", "list"]);
    assert!(list_out.contains("myproject"));
    // The disk path should not have .git.git
    assert!(home.join("repos/myproject.git").exists());
}

#[test]
fn test_repo_list_empty() {
    let home = mgs_home();
    let out = mgs_cmd(&home, &["repo", "list"]);
    assert!(out.contains("No repositories found"));
}

#[test]
fn test_repo_remove_nonexistent() {
    let home = mgs_home();
    let out = mgs_cmd(&home, &["repo", "remove", "nope"]);
    assert!(out.contains("not found"));
}
