use mgs::db::Database;
use tempfile::TempDir;

fn test_db() -> (TempDir, Database) {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let db = Database::open(&db_path).unwrap();
    (tmp, db)
}

// --- Users ---

#[test]
fn test_create_and_find_user() {
    let (_tmp, db) = test_db();
    let user = db.create_user("alice", "token_alice").unwrap();
    assert_eq!(user.username, "alice");
    assert!(user.id > 0);
    assert!(!user.created_at.is_empty());

    let found = db.find_user_by_username("alice").unwrap().unwrap();
    assert_eq!(found.id, user.id);
    assert_eq!(found.username, "alice");
}

#[test]
fn test_find_user_by_id() {
    let (_tmp, db) = test_db();
    let user = db.create_user("bob", "token_bob").unwrap();
    let found = db.find_user_by_id(user.id).unwrap().unwrap();
    assert_eq!(found.username, "bob");
}

#[test]
fn test_find_user_not_found() {
    let (_tmp, db) = test_db();
    assert!(db.find_user_by_username("nobody").unwrap().is_none());
    assert!(db.find_user_by_id(999).unwrap().is_none());
}

#[test]
fn test_create_user_duplicate() {
    let (_tmp, db) = test_db();
    db.create_user("alice", "token_alice").unwrap();
    assert!(db.create_user("alice", "token_alice").is_err());
}

#[test]
fn test_list_users() {
    let (_tmp, db) = test_db();
    assert!(db.list_users().unwrap().is_empty());

    db.create_user("charlie", "token_charlie").unwrap();
    db.create_user("alice", "token_alice").unwrap();
    db.create_user("bob", "token_bob").unwrap();

    let users = db.list_users().unwrap();
    assert_eq!(users.len(), 3);
    assert_eq!(users[0].username, "alice"); // ordered by username
    assert_eq!(users[1].username, "bob");
    assert_eq!(users[2].username, "charlie");
}

#[test]
fn test_delete_user() {
    let (_tmp, db) = test_db();
    db.create_user("alice", "token_alice").unwrap();
    assert!(db.delete_user("alice").unwrap());
    assert!(!db.delete_user("alice").unwrap());
    assert!(db.find_user_by_username("alice").unwrap().is_none());
}

#[test]
fn test_delete_user_cascades_keys() {
    let (_tmp, db) = test_db();
    let user = db.create_user("alice", "token_alice").unwrap();
    db.add_ssh_key(user.id, "ssh-ed25519", "AAAA1234", "SHA256:abc")
        .unwrap();

    db.delete_user("alice").unwrap();
    assert!(db.list_ssh_keys(user.id).unwrap().is_empty());
}

// --- SSH Keys ---

#[test]
fn test_add_and_list_keys() {
    let (_tmp, db) = test_db();
    let user = db.create_user("alice", "token_alice").unwrap();

    let k1 = db
        .add_ssh_key(user.id, "ssh-ed25519", "AAAA111", "SHA256:aaa")
        .unwrap();
    let k2 = db
        .add_ssh_key(user.id, "ssh-rsa", "AAAA222", "SHA256:bbb")
        .unwrap();

    assert_eq!(k1.user_id, user.id);
    assert_eq!(k1.key_type, "ssh-ed25519");
    assert_eq!(k2.key_type, "ssh-rsa");

    let keys = db.list_ssh_keys(user.id).unwrap();
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0].fingerprint, "SHA256:aaa");
    assert_eq!(keys[1].fingerprint, "SHA256:bbb");
}

#[test]
fn test_list_keys_empty() {
    let (_tmp, db) = test_db();
    let user = db.create_user("alice", "token_alice").unwrap();
    assert!(db.list_ssh_keys(user.id).unwrap().is_empty());
}

#[test]
fn test_delete_key() {
    let (_tmp, db) = test_db();
    let user = db.create_user("alice", "token_alice").unwrap();
    db.add_ssh_key(user.id, "ssh-ed25519", "AAAA", "SHA256:x")
        .unwrap();

    assert!(db.delete_ssh_key("SHA256:x").unwrap());
    assert!(!db.delete_ssh_key("SHA256:x").unwrap());
    assert!(db.list_ssh_keys(user.id).unwrap().is_empty());
}

#[test]
fn test_add_key_duplicate_fingerprint() {
    let (_tmp, db) = test_db();
    let user = db.create_user("alice", "token_alice").unwrap();
    db.add_ssh_key(user.id, "ssh-ed25519", "AAAA", "SHA256:dup")
        .unwrap();
    assert!(
        db.add_ssh_key(user.id, "ssh-rsa", "BBBB", "SHA256:dup")
            .is_err()
    );
}

#[test]
fn test_find_user_by_fingerprint() {
    let (_tmp, db) = test_db();
    let user = db.create_user("alice", "token_alice").unwrap();
    db.add_ssh_key(user.id, "ssh-ed25519", "AAAA", "SHA256:findme")
        .unwrap();

    let found = db
        .find_user_by_fingerprint("SHA256:findme")
        .unwrap()
        .unwrap();
    assert_eq!(found.username, "alice");
    assert!(
        db.find_user_by_fingerprint("SHA256:nope")
            .unwrap()
            .is_none()
    );
}

// --- Tokens ---

#[test]
fn test_find_user_by_token() {
    let (_tmp, db) = test_db();
    db.create_user("alice", "token_alice").unwrap();

    let found = db.find_user_by_token("token_alice").unwrap().unwrap();
    assert_eq!(found.username, "alice");
    assert_eq!(found.token.as_deref(), Some("token_alice"));
}

#[test]
fn test_find_user_by_token_not_found() {
    let (_tmp, db) = test_db();
    assert!(db.find_user_by_token("nonexistent").unwrap().is_none());
}

#[test]
fn test_set_user_token() {
    let (_tmp, db) = test_db();
    db.create_user("alice", "old_token").unwrap();

    assert!(db.set_user_token("alice", "new_token").unwrap());
    let user = db.find_user_by_username("alice").unwrap().unwrap();
    assert_eq!(user.token.as_deref(), Some("new_token"));
}

#[test]
fn test_set_user_token_nonexistent() {
    let (_tmp, db) = test_db();
    assert!(!db.set_user_token("nobody", "token").unwrap());
}

#[test]
fn test_generate_token_length() {
    let token = mgs::generate_token();
    assert_eq!(token.len(), 64);
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_generate_token_unique() {
    let t1 = mgs::generate_token();
    let t2 = mgs::generate_token();
    assert_ne!(t1, t2);
}

// --- Repositories ---

#[test]
fn test_create_and_find_repo() {
    let (_tmp, db) = test_db();
    let user = db.create_user("alice", "token_alice").unwrap();
    let repo = db.create_repo("team/project", user.id).unwrap();

    assert_eq!(repo.name, "team/project");
    assert_eq!(repo.owner_id, user.id);

    let found = db.find_repo("team/project").unwrap().unwrap();
    assert_eq!(found.id, repo.id);
}

#[test]
fn test_find_repo_not_found() {
    let (_tmp, db) = test_db();
    assert!(db.find_repo("nonexistent").unwrap().is_none());
}

#[test]
fn test_create_repo_duplicate() {
    let (_tmp, db) = test_db();
    let user = db.create_user("alice", "token_alice").unwrap();
    db.create_repo("myrepo", user.id).unwrap();
    assert!(db.create_repo("myrepo", user.id).is_err());
}

#[test]
fn test_list_repos() {
    let (_tmp, db) = test_db();
    let user = db.create_user("alice", "token_alice").unwrap();
    assert!(db.list_repos().unwrap().is_empty());

    db.create_repo("beta", user.id).unwrap();
    db.create_repo("alpha", user.id).unwrap();
    db.create_repo("gamma", user.id).unwrap();

    let repos = db.list_repos().unwrap();
    assert_eq!(repos.len(), 3);
    assert_eq!(repos[0].name, "alpha"); // ordered by name
    assert_eq!(repos[1].name, "beta");
    assert_eq!(repos[2].name, "gamma");
}

#[test]
fn test_delete_repo() {
    let (_tmp, db) = test_db();
    let user = db.create_user("alice", "token_alice").unwrap();
    db.create_repo("myrepo", user.id).unwrap();

    assert!(db.delete_repo("myrepo").unwrap());
    assert!(!db.delete_repo("myrepo").unwrap());
    assert!(db.find_repo("myrepo").unwrap().is_none());
}

// --- Token Persistence ---

#[test]
fn test_token_persists_across_reopen() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let db1 = Database::open(&db_path).unwrap();
    db1.create_user("alice", "persist_token").unwrap();
    drop(db1);

    let db2 = Database::open(&db_path).unwrap();
    let user = db2.find_user_by_username("alice").unwrap().unwrap();
    assert_eq!(user.token.as_deref(), Some("persist_token"));
}

#[test]
fn test_token_updated_persists() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let db1 = Database::open(&db_path).unwrap();
    db1.create_user("alice", "old").unwrap();
    db1.set_user_token("alice", "new").unwrap();
    drop(db1);

    let db2 = Database::open(&db_path).unwrap();
    let user = db2.find_user_by_token("new").unwrap().unwrap();
    assert_eq!(user.username, "alice");
    assert!(db2.find_user_by_token("old").unwrap().is_none());
}

#[test]
fn test_multiple_users_different_tokens() {
    let (_tmp, db) = test_db();
    db.create_user("alice", "token_a").unwrap();
    db.create_user("bob", "token_b").unwrap();
    db.create_user("charlie", "token_c").unwrap();

    assert_eq!(
        db.find_user_by_token("token_a").unwrap().unwrap().username,
        "alice"
    );
    assert_eq!(
        db.find_user_by_token("token_b").unwrap().unwrap().username,
        "bob"
    );
    assert_eq!(
        db.find_user_by_token("token_c").unwrap().unwrap().username,
        "charlie"
    );
}

#[test]
fn test_token_deleted_with_user() {
    let (_tmp, db) = test_db();
    db.create_user("alice", "token_del").unwrap();
    assert!(db.find_user_by_token("token_del").unwrap().is_some());

    db.delete_user("alice").unwrap();
    assert!(db.find_user_by_token("token_del").unwrap().is_none());
}

#[test]
fn test_user_token_is_none_for_null() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let db = Database::open(&db_path).unwrap();
    // Insert directly without token (simulating legacy user)
    db.create_user("legacy", "legacy_token").unwrap();
    db.set_user_token("legacy", "").unwrap();
    // Empty string is stored, not null
    let user = db.find_user_by_username("legacy").unwrap().unwrap();
    assert_eq!(user.token.as_deref(), Some(""));
}

#[test]
fn test_set_user_token_to_same_value() {
    let (_tmp, db) = test_db();
    db.create_user("alice", "same_token").unwrap();
    assert!(db.set_user_token("alice", "same_token").unwrap());
    let user = db.find_user_by_token("same_token").unwrap().unwrap();
    assert_eq!(user.username, "alice");
}

#[test]
fn test_find_user_by_token_after_regenerate() {
    let (_tmp, db) = test_db();
    db.create_user("alice", "original").unwrap();

    let new_token = mgs::generate_token();
    db.set_user_token("alice", &new_token).unwrap();

    assert!(db.find_user_by_token("original").unwrap().is_none());
    let user = db.find_user_by_token(&new_token).unwrap().unwrap();
    assert_eq!(user.username, "alice");
}

// --- Open ---

#[test]
fn test_open_is_idempotent() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let db1 = Database::open(&db_path).unwrap();
    db1.create_user("alice", "token_alice").unwrap();
    drop(db1);

    let db2 = Database::open(&db_path).unwrap();
    let user = db2.find_user_by_username("alice").unwrap().unwrap();
    assert_eq!(user.username, "alice");
}

#[test]
fn test_open_preserves_tokens() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");

    let db = Database::open(&db_path).unwrap();
    db.create_user("alice", "tok1").unwrap();
    db.create_user("bob", "tok2").unwrap();
    drop(db);

    let db2 = Database::open(&db_path).unwrap();
    assert_eq!(
        db2.find_user_by_token("tok1").unwrap().unwrap().username,
        "alice"
    );
    assert_eq!(
        db2.find_user_by_token("tok2").unwrap().unwrap().username,
        "bob"
    );
}
