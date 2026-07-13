use super::*;

#[test]
fn test_full_workflow() {
    let home = mgs_home();
    mgs_cmd(&home, &["init"]);

    // Create users
    let key1 = generate_test_key(&home, "admin_key");
    mgs_cmd(
        &home,
        &["user", "add", "admin", "--key", key1.to_str().unwrap()],
    );

    let key2 = generate_test_key(&home, "dev_key");
    mgs_cmd(
        &home,
        &["user", "add", "developer", "--key", key2.to_str().unwrap()],
    );

    // Create repo
    let out = mgs_cmd(
        &home,
        &["repo", "create", "team/backend", "--owner", "admin"],
    );
    assert!(out.contains("Created repository 'team/backend'"));

    // Verify user list
    let users = mgs_cmd(&home, &["user", "list"]);
    assert!(users.contains("admin"));
    assert!(users.contains("developer"));

    // Verify repo list
    let repos = mgs_cmd(&home, &["repo", "list"]);
    assert!(repos.contains("team/backend"));
    assert!(repos.contains("owner: admin"));

    // Remove user and verify
    mgs_cmd(&home, &["user", "remove", "developer"]);
    let users_after = mgs_cmd(&home, &["user", "list"]);
    assert!(!users_after.contains("developer"));
    assert!(users_after.contains("admin"));
}
