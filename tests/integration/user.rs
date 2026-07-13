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

#[test]
fn test_user_add_shows_token() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "token_key");
    let out = mgs_cmd(
        &home,
        &[
            "user",
            "add",
            "tokenuser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    assert!(out.contains("HTTP token:"));
    // Token should be 64 hex chars
    let token_line = out.lines().find(|l| l.starts_with("HTTP token:")).unwrap();
    let token = token_line.strip_prefix("HTTP token: ").unwrap().trim();
    assert_eq!(token.len(), 64);
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_token_show() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "show_key");
    let out = mgs_cmd(
        &home,
        &[
            "user",
            "add",
            "showuser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    // Extract token from add output
    let token_line = out.lines().find(|l| l.starts_with("HTTP token:")).unwrap();
    let expected_token = token_line.strip_prefix("HTTP token: ").unwrap().trim();

    // Show token
    let show_out = mgs_cmd(&home, &["user", "token", "show", "showuser"]);
    assert_eq!(show_out.trim(), expected_token);
}

#[test]
fn test_token_regenerate() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "regen_key");
    mgs_cmd(
        &home,
        &[
            "user",
            "add",
            "regenuser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );

    // Get original token
    let original = mgs_cmd(&home, &["user", "token", "show", "regenuser"]);
    let original = original.trim();

    // Regenerate
    let regen_out = mgs_cmd(&home, &["user", "token", "regenerate", "regenuser"]);
    assert!(regen_out.contains("New token for 'regenuser':"));
    let new_token = regen_out
        .lines()
        .find(|l| l.starts_with("New token"))
        .unwrap()
        .split(':')
        .last()
        .unwrap()
        .trim();
    assert_ne!(new_token, original);
    assert_eq!(new_token.len(), 64);

    // Verify new token works
    let show_out = mgs_cmd(&home, &["user", "token", "show", "regenuser"]);
    assert_eq!(show_out.trim(), new_token);
}

#[test]
fn test_token_show_nonexistent_user() {
    let home = mgs_home();
    let err = mgs_cmd_fails(&home, &["user", "token", "show", "ghost"]);
    assert!(err.contains("not found"));
}

#[test]
fn test_token_regenerate_nonexistent_user() {
    let home = mgs_home();
    let err = mgs_cmd_fails(&home, &["user", "token", "regenerate", "ghost"]);
    assert!(err.contains("not found"));
}

#[test]
fn test_user_add_multiple_users_have_different_tokens() {
    let home = mgs_home();
    let key1 = generate_test_key(&home, "multi_key1");
    let key2 = generate_test_key(&home, "multi_key2");

    let out1 = mgs_cmd(
        &home,
        &["user", "add", "user1", "--key", key1.to_str().unwrap()],
    );
    let out2 = mgs_cmd(
        &home,
        &["user", "add", "user2", "--key", key2.to_str().unwrap()],
    );

    let token1 = out1
        .lines()
        .find(|l| l.starts_with("HTTP token:"))
        .unwrap()
        .strip_prefix("HTTP token: ")
        .unwrap()
        .trim();
    let token2 = out2
        .lines()
        .find(|l| l.starts_with("HTTP token:"))
        .unwrap()
        .strip_prefix("HTTP token: ")
        .unwrap()
        .trim();

    assert_ne!(token1, token2);
    assert_eq!(token1.len(), 64);
    assert_eq!(token2.len(), 64);
}

#[test]
fn test_token_survives_user_list() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "survive_key");
    mgs_cmd(
        &home,
        &[
            "user",
            "add",
            "surviveuser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );

    let token_before = mgs_cmd(&home, &["user", "token", "show", "surviveuser"]);
    mgs_cmd(&home, &["user", "list"]);
    let token_after = mgs_cmd(&home, &["user", "token", "show", "surviveuser"]);
    assert_eq!(token_before.trim(), token_after.trim());
}

#[test]
fn test_token_survives_key_operations() {
    let home = mgs_home();
    let key1 = generate_test_key(&home, "keyop_key1");
    mgs_cmd(
        &home,
        &["user", "add", "keyopuser", "--key", key1.to_str().unwrap()],
    );

    let token_before = mgs_cmd(&home, &["user", "token", "show", "keyopuser"]);

    let key2 = generate_test_key(&home, "keyop_key2");
    mgs_cmd(
        &home,
        &[
            "user",
            "key",
            "add",
            "keyopuser",
            "--key",
            key2.to_str().unwrap(),
        ],
    );

    let token_after = mgs_cmd(&home, &["user", "token", "show", "keyopuser"]);
    assert_eq!(token_before.trim(), token_after.trim());
}

#[test]
fn test_user_add_token_format() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "fmt_key");
    let out = mgs_cmd(
        &home,
        &[
            "user",
            "add",
            "fmtuser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );

    let token_line = out.lines().find(|l| l.starts_with("HTTP token:")).unwrap();
    let token = token_line.strip_prefix("HTTP token: ").unwrap().trim();
    assert_eq!(token.len(), 64);
    assert!(
        token.chars().all(|c| c.is_ascii_hexdigit()),
        "token should be hex: {}",
        token
    );
}
