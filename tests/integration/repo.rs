use super::*;

#[test]
fn test_repo_create_and_list() {
    let home = mgs_home();
    mgs_cmd(&home, &["init"]);

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

    let list_out = mgs_cmd(&home, &["repo", "list"]);
    assert!(list_out.contains("team/project"));
    assert!(list_out.contains("owner: owner1"));
}

#[test]
fn test_repo_remove() {
    let home = mgs_home();
    mgs_cmd(&home, &["init"]);

    let key_path = generate_test_key(&home, "rr_key");
    mgs_cmd(
        &home,
        &["user", "add", "owner2", "--key", key_path.to_str().unwrap()],
    );
    mgs_cmd(&home, &["repo", "create", "my/repo", "--owner", "owner2"]);

    let out = mgs_cmd(&home, &["repo", "remove", "my/repo"]);
    assert!(out.contains("Removed repository 'my/repo'"));

    let list_out = mgs_cmd(&home, &["repo", "list"]);
    assert!(list_out.contains("No repositories found"));
}

#[test]
fn test_repo_create_nonexistent_owner() {
    let home = mgs_home();
    mgs_cmd(&home, &["init"]);

    let err = mgs_cmd_fails(&home, &["repo", "create", "x/y", "--owner", "ghost"]);
    assert!(err.contains("not found"));
}
