use super::*;
use std::fs;

#[test]
fn test_file_modification_roundtrip() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "modifier");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &[
            "user",
            "add",
            "modifier",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs(&home, &["repo", "create", "modrepo", "--owner", "modifier"]);

    let wrapper = create_ssh_wrapper(&home, &fp);
    let work = home.join("mod_work");
    fs::create_dir_all(&work).unwrap();

    git_cmd(&work, &wrapper, &["clone", "mgs:modrepo.git", "."]);
    git_config_user(&work);

    // Create file, push
    git_commit(&work, "data.txt", "version 1", "v1");
    let branch = current_branch(&work);
    git_cmd(&work, &wrapper, &["push", "origin", &branch]);

    // Modify file, push
    git_commit(&work, "data.txt", "version 2", "v2");
    git_cmd(&work, &wrapper, &["push", "origin", &branch]);

    // Delete file, push
    fs::remove_file(work.join("data.txt")).unwrap();
    Command::new("git")
        .current_dir(&work)
        .args(["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(&work)
        .args(["commit", "-m", "delete"])
        .output()
        .unwrap();
    git_cmd(&work, &wrapper, &["push", "origin", &branch]);

    // Clone and verify
    let verify = home.join("mod_verify");
    fs::create_dir_all(&verify).unwrap();
    git_cmd(&verify, &wrapper, &["clone", "mgs:modrepo.git", "."]);

    assert!(!verify.join("data.txt").exists());

    let log = Command::new("git")
        .current_dir(&verify)
        .args(["log", "--oneline"])
        .output()
        .unwrap();
    let log_str = String::from_utf8_lossy(&log.stdout);
    assert!(log_str.contains("delete"));
    assert!(log_str.contains("v2"));
    assert!(log_str.contains("v1"));
}

#[test]
fn test_binary_file_roundtrip() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "binary_user");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &[
            "user",
            "add",
            "binary_user",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs(
        &home,
        &["repo", "create", "binrepo", "--owner", "binary_user"],
    );

    let wrapper = create_ssh_wrapper(&home, &fp);
    let work = home.join("bin_work");
    fs::create_dir_all(&work).unwrap();

    git_cmd(&work, &wrapper, &["clone", "mgs:binrepo.git", "."]);
    git_config_user(&work);

    // Create binary file (random bytes)
    let binary_data: Vec<u8> = (0..256).map(|i| i as u8).collect();
    fs::write(work.join("data.bin"), &binary_data).unwrap();

    Command::new("git")
        .current_dir(&work)
        .args(["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(&work)
        .args(["commit", "-m", "binary"])
        .output()
        .unwrap();
    let branch = current_branch(&work);
    git_cmd(&work, &wrapper, &["push", "origin", &branch]);

    // Clone and verify binary content matches
    let verify = home.join("bin_verify");
    fs::create_dir_all(&verify).unwrap();
    git_cmd(&verify, &wrapper, &["clone", "mgs:binrepo.git", "."]);

    let cloned_data = fs::read(verify.join("data.bin")).unwrap();
    assert_eq!(cloned_data, binary_data);
}

#[test]
fn test_special_filenames() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "special_user");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &[
            "user",
            "add",
            "special_user",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs(
        &home,
        &["repo", "create", "specialrepo", "--owner", "special_user"],
    );

    let wrapper = create_ssh_wrapper(&home, &fp);
    let work = home.join("special_work");
    fs::create_dir_all(&work).unwrap();

    git_cmd(&work, &wrapper, &["clone", "mgs:specialrepo.git", "."]);
    git_config_user(&work);

    // Create files with special names
    let files = vec![
        ("file with spaces.txt", "spaces"),
        ("file-with-dashes.txt", "dashes"),
        ("file_with_underscores.txt", "underscores"),
        ("file.with.dots.txt", "dots"),
        ("UPPERCASE.TXT", "upper"),
        ("CamelCase.txt", "camel"),
        ("123numeric.txt", "numeric"),
    ];

    for (name, content) in &files {
        fs::write(work.join(name), *content).unwrap();
    }

    Command::new("git")
        .current_dir(&work)
        .args(["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(&work)
        .args(["commit", "-m", "special files"])
        .output()
        .unwrap();
    let branch = current_branch(&work);
    git_cmd(&work, &wrapper, &["push", "origin", &branch]);

    // Clone and verify
    let verify = home.join("special_verify");
    fs::create_dir_all(&verify).unwrap();
    git_cmd(&verify, &wrapper, &["clone", "mgs:specialrepo.git", "."]);

    for (name, content) in &files {
        let path = verify.join(name);
        assert!(path.exists(), "{} should exist", name);
        assert_eq!(fs::read_to_string(&path).unwrap(), *content);
    }
}

#[test]
fn test_deep_directory_structure() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "deep_user");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &[
            "user",
            "add",
            "deep_user",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs(
        &home,
        &["repo", "create", "deeprepo", "--owner", "deep_user"],
    );

    let wrapper = create_ssh_wrapper(&home, &fp);
    let work = home.join("deep_work");
    fs::create_dir_all(&work).unwrap();

    git_cmd(&work, &wrapper, &["clone", "mgs:deeprepo.git", "."]);
    git_config_user(&work);

    // Create deep nested structure
    let deep_path = work.join("a/b/c/d/e/f");
    fs::create_dir_all(&deep_path).unwrap();
    fs::write(deep_path.join("deep.txt"), "deeply nested").unwrap();

    Command::new("git")
        .current_dir(&work)
        .args(["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(&work)
        .args(["commit", "-m", "deep structure"])
        .output()
        .unwrap();
    let branch = current_branch(&work);
    git_cmd(&work, &wrapper, &["push", "origin", &branch]);

    // Clone and verify
    let verify = home.join("deep_verify");
    fs::create_dir_all(&verify).unwrap();
    git_cmd(&verify, &wrapper, &["clone", "mgs:deeprepo.git", "."]);

    let deep_file = verify.join("a/b/c/d/e/f/deep.txt");
    assert!(deep_file.exists());
    assert_eq!(fs::read_to_string(deep_file).unwrap(), "deeply nested");
}

#[test]
fn test_large_file_roundtrip() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "large_user");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &[
            "user",
            "add",
            "large_user",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs(
        &home,
        &["repo", "create", "largerepo", "--owner", "large_user"],
    );

    let wrapper = create_ssh_wrapper(&home, &fp);
    let work = home.join("large_work");
    fs::create_dir_all(&work).unwrap();

    git_cmd(&work, &wrapper, &["clone", "mgs:largerepo.git", "."]);
    git_config_user(&work);

    // Create a 1MB file with known content
    let large_data: Vec<u8> = (0..1_048_576).map(|i| (i % 256) as u8).collect();
    fs::write(work.join("large.bin"), &large_data).unwrap();

    Command::new("git")
        .current_dir(&work)
        .args(["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(&work)
        .args(["commit", "-m", "large file"])
        .output()
        .unwrap();
    let branch = current_branch(&work);
    git_cmd(&work, &wrapper, &["push", "origin", &branch]);

    // Clone and verify
    let verify = home.join("large_verify");
    fs::create_dir_all(&verify).unwrap();
    git_cmd(&verify, &wrapper, &["clone", "mgs:largerepo.git", "."]);

    let cloned = fs::read(verify.join("large.bin")).unwrap();
    assert_eq!(cloned.len(), 1_048_576);
    assert_eq!(cloned, large_data);
}

#[test]
fn test_many_small_commits() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "many_user");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &[
            "user",
            "add",
            "many_user",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs(
        &home,
        &["repo", "create", "manyrepo", "--owner", "many_user"],
    );

    let wrapper = create_ssh_wrapper(&home, &fp);
    let work = home.join("many_work");
    fs::create_dir_all(&work).unwrap();

    git_cmd(&work, &wrapper, &["clone", "mgs:manyrepo.git", "."]);
    git_config_user(&work);

    // Push 20 commits one at a time
    let branch = current_branch(&work);
    for i in 0..20 {
        git_commit(
            &work,
            &format!("file_{}.txt", i),
            &format!("content {}", i),
            &format!("commit {}", i),
        );
        git_cmd(&work, &wrapper, &["push", "origin", &branch]);
    }

    // Clone and verify all files exist
    let verify = home.join("many_verify");
    fs::create_dir_all(&verify).unwrap();
    git_cmd(&verify, &wrapper, &["clone", "mgs:manyrepo.git", "."]);

    for i in 0..20 {
        let path = verify.join(format!("file_{}.txt", i));
        assert!(path.exists(), "file_{}.txt should exist", i);
        assert_eq!(fs::read_to_string(&path).unwrap(), format!("content {}", i));
    }

    // Verify commit count
    let log = Command::new("git")
        .current_dir(&verify)
        .args(["rev-list", "--count", "HEAD"])
        .output()
        .unwrap();
    let count: u32 = String::from_utf8_lossy(&log.stdout).trim().parse().unwrap();
    assert_eq!(count, 20);
}
