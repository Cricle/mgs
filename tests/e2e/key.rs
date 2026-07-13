use super::*;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_user_remove_key_still_works() {
    let home = test_home();
    mgs(&home, &["init"]);

    // Create user with two keys
    let key1 = generate_key(&home, "rk_key1");
    let fp1 = key_fingerprint(&key1);
    mgs(
        &home,
        &["user", "add", "rkuser", "--key", key1.to_str().unwrap()],
    );

    let key2 = generate_key(&home, "rk_key2");
    let fp2 = key_fingerprint(&key2);
    mgs(
        &home,
        &[
            "user",
            "key",
            "add",
            "rkuser",
            "--key",
            key2.to_str().unwrap(),
        ],
    );

    mgs(&home, &["repo", "create", "rkrepo", "--owner", "rkuser"]);

    // Both keys can access
    let wrapper1 = create_ssh_wrapper(&home, &fp1);
    let wrapper2 = create_ssh_wrapper(&home, &fp2);

    let dir1 = home.join("rk_work1");
    fs::create_dir_all(&dir1).unwrap();
    git_cmd(&dir1, &wrapper1, &["clone", "mgs:rkrepo.git", "."]);

    let dir2 = home.join("rk_work2");
    fs::create_dir_all(&dir2).unwrap();
    git_cmd(&dir2, &wrapper2, &["clone", "mgs:rkrepo.git", "."]);

    // Remove key1
    mgs(&home, &["user", "key", "remove", &fp1]);

    // Key1 should no longer work — verify mgs-ssh actually fails
    let dir3 = home.join("rk_work3");
    fs::create_dir_all(&dir3).unwrap();
    let mgs_path = PathBuf::from(env!("CARGO_BIN_EXE_mgs"));
    let mgs_ssh = mgs_path.parent().unwrap().join("mgs-ssh");
    let mgs_ssh_output = Command::new(&mgs_ssh)
        .arg(&fp1)
        .env("SSH_ORIGINAL_COMMAND", "git-upload-pack 'rkrepo.git'")
        .env("MGS_HOME", home.to_str().unwrap())
        .output()
        .unwrap();
    assert!(
        !mgs_ssh_output.status.success(),
        "mgs-ssh should fail for removed key, stderr: {}",
        String::from_utf8_lossy(&mgs_ssh_output.stderr)
    );

    // Key2 should still work
    let dir4 = home.join("rk_work4");
    fs::create_dir_all(&dir4).unwrap();
    git_cmd(&dir4, &wrapper2, &["clone", "mgs:rkrepo.git", "."]);
    assert!(dir4.join(".git").exists());
}
