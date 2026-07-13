use super::*;

#[test]
fn test_full_workflow() {
    let home = mgs_home();

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

    // Add second key for developer
    let key3 = generate_test_key(&home, "dev_key2");
    mgs_cmd(
        &home,
        &[
            "user",
            "key",
            "add",
            "developer",
            "--key",
            key3.to_str().unwrap(),
        ],
    );
    let keys = mgs_cmd(&home, &["user", "key", "list", "developer"]);
    let key_count = keys.trim().lines().count();
    assert_eq!(key_count, 2);

    // Remove user and verify
    mgs_cmd(&home, &["user", "remove", "developer"]);
    let users_after = mgs_cmd(&home, &["user", "list"]);
    assert!(!users_after.contains("developer"));
    assert!(users_after.contains("admin"));

    // Remove repo
    mgs_cmd(&home, &["repo", "remove", "team/backend"]);
    let repos_after = mgs_cmd(&home, &["repo", "list"]);
    assert!(repos_after.contains("No repositories found"));
}

#[test]
fn test_multiple_repos_workflow() {
    let home = mgs_home();

    let key = generate_test_key(&home, "multi_key");
    mgs_cmd(
        &home,
        &["user", "add", "multiowner", "--key", key.to_str().unwrap()],
    );

    // Create multiple repos
    for name in &["alpha", "beta", "gamma"] {
        mgs_cmd(&home, &["repo", "create", name, "--owner", "multiowner"]);
    }

    let list = mgs_cmd(&home, &["repo", "list"]);
    assert!(list.contains("alpha"));
    assert!(list.contains("beta"));
    assert!(list.contains("gamma"));

    // Remove one
    mgs_cmd(&home, &["repo", "remove", "beta"]);
    let list_after = mgs_cmd(&home, &["repo", "list"]);
    assert!(list_after.contains("alpha"));
    assert!(!list_after.contains("beta"));
    assert!(list_after.contains("gamma"));
}

#[test]
fn test_auto_init_on_first_command() {
    let home = mgs_home();
    // No init call — just run a command directly
    let out = mgs_cmd(&home, &["user", "list"]);
    assert!(out.contains("No users found"));
    // Data dir should have been auto-created
    assert!(home.join("mgs.db").exists());
    assert!(home.join("repos").exists());
}
