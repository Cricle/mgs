use super::*;

#[test]
fn test_user_add_and_list() {
    let home = mgs_home();
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
    let err = mgs_cmd_fails(
        &home,
        &["user", "add", "u1", "--key", "/tmp/nonexistent-key.pub"],
    );
    assert!(err.contains("failed to read key file"));
}

#[test]
fn test_user_list_empty() {
    let home = mgs_home();
    let out = mgs_cmd(&home, &["user", "list"]);
    assert!(out.contains("No users found"));
}

#[test]
fn test_user_remove_nonexistent() {
    let home = mgs_home();
    let out = mgs_cmd(&home, &["user", "remove", "ghost"]);
    assert!(out.contains("not found"));
}

#[test]
fn test_user_add_invalid_username() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "bad_name_key");
    let err = mgs_cmd_fails(
        &home,
        &[
            "user",
            "add",
            "bad name!",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    assert!(err.contains("invalid character"));
}

#[test]
fn test_key_add_and_list_and_remove() {
    let home = mgs_home();
    let key1 = generate_test_key(&home, "key1");
    mgs_cmd(
        &home,
        &["user", "add", "keyuser", "--key", key1.to_str().unwrap()],
    );

    // Add second key
    let key2 = generate_test_key(&home, "key2");
    mgs_cmd(
        &home,
        &[
            "user",
            "key",
            "add",
            "keyuser",
            "--key",
            key2.to_str().unwrap(),
        ],
    );

    let list_out = mgs_cmd(&home, &["user", "key", "list", "keyuser"]);
    assert!(list_out.contains("SHA256:"));
    // Should have 2 lines of key output
    let lines: Vec<&str> = list_out.trim().lines().collect();
    assert_eq!(lines.len(), 2);

    // Remove first key by fingerprint
    let fp1 = list_out
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap();
    mgs_cmd(&home, &["user", "key", "remove", fp1]);

    let list_after = mgs_cmd(&home, &["user", "key", "list", "keyuser"]);
    let lines_after: Vec<&str> = list_after.trim().lines().collect();
    assert_eq!(lines_after.len(), 1);
}

#[test]
fn test_key_list_empty() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "empty_key");
    mgs_cmd(
        &home,
        &[
            "user",
            "add",
            "emptykeyuser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    // Remove the only key
    let list_out = mgs_cmd(&home, &["user", "key", "list", "emptykeyuser"]);
    let fp = list_out
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap();
    mgs_cmd(&home, &["user", "key", "remove", fp]);

    let out = mgs_cmd(&home, &["user", "key", "list", "emptykeyuser"]);
    assert!(out.contains("No keys"));
}

#[test]
fn test_key_remove_nonexistent() {
    let home = mgs_home();
    let out = mgs_cmd(&home, &["user", "key", "remove", "SHA256:nonexistent"]);
    assert!(out.contains("not found"));
}

#[test]
fn test_key_add_nonexistent_user() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "no_user_key");
    let err = mgs_cmd_fails(
        &home,
        &[
            "user",
            "key",
            "add",
            "nobody",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    assert!(err.contains("not found"));
}
