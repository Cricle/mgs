use super::*;

#[test]
fn test_init() {
    let home = mgs_home();
    let out = mgs_cmd(&home, &["init"]);
    assert!(out.contains("Initialized"));
    assert!(home.join("mgs.db").exists());
    assert!(home.join("repos").exists());
}

#[test]
fn test_init_idempotent() {
    let home = mgs_home();
    let out1 = mgs_cmd(&home, &["init"]);
    assert!(out1.contains("Initialized"));

    let out2 = mgs_cmd(&home, &["init"]);
    assert!(out2.contains("already initialized"));
}
