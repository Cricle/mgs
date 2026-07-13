use super::*;

#[test]
fn test_user_add_and_list() {
    let home = mgs_home();
    mgs_cmd(&home, &["init"]);

    let key_path = generate_test_key(&home, "test_key");
    let out = mgs_cmd(
        &home,
        &["user", "add", "alice", "--key", key_path.to_str().unwrap()],
    );
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
    mgs_cmd(
        &home,
        &["user", "add", "bob", "--key", key_path.to_str().unwrap()],
    );

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
    mgs_cmd(
        &home,
        &[
            "user",
            "add",
            "charlie",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );

    let err = mgs_cmd_fails(
        &home,
        &[
            "user",
            "add",
            "charlie",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    assert!(err.contains("already exists"));
}

#[test]
fn test_user_add_nonexistent_key_file() {
    let home = mgs_home();
    mgs_cmd(&home, &["init"]);

    let err = mgs_cmd_fails(
        &home,
        &["user", "add", "u1", "--key", "/tmp/nonexistent-key.pub"],
    );
    assert!(err.contains("failed to read key file"));
}
