use super::*;
use std::fs;

#[test]
fn test_nested_repo_name() {
    let home = test_home();

    let key_path = generate_key(&home, "nested_user");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &[
            "user",
            "add",
            "nested_user",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs(
        &home,
        &[
            "repo",
            "create",
            "team/backend/api",
            "--owner",
            "nested_user",
        ],
    );

    let wrapper = create_ssh_wrapper(&home, &fp);
    let work = home.join("nested_work");
    fs::create_dir_all(&work).unwrap();

    git_cmd(&work, &wrapper, &["clone", "mgs:team/backend/api.git", "."]);
    git_config_user(&work);
    git_commit(&work, "lib.rs", "fn main() {}", "init");
    let branch = current_branch(&work);
    git_cmd(&work, &wrapper, &["push", "origin", &branch]);

    let verify = home.join("nested_verify");
    fs::create_dir_all(&verify).unwrap();
    git_cmd(
        &verify,
        &wrapper,
        &["clone", "mgs:team/backend/api.git", "."],
    );
    assert_eq!(
        fs::read_to_string(verify.join("lib.rs")).unwrap(),
        "fn main() {}"
    );
}

#[test]
fn test_multiple_repos_same_user() {
    let home = test_home();

    let key_path = generate_key(&home, "multi_repo_user");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &[
            "user",
            "add",
            "multi_repo_user",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );

    mgs(
        &home,
        &["repo", "create", "repo-alpha", "--owner", "multi_repo_user"],
    );
    mgs(
        &home,
        &["repo", "create", "repo-beta", "--owner", "multi_repo_user"],
    );
    mgs(
        &home,
        &["repo", "create", "repo-gamma", "--owner", "multi_repo_user"],
    );

    let wrapper = create_ssh_wrapper(&home, &fp);

    // Push to each repo
    for name in &["repo-alpha", "repo-beta", "repo-gamma"] {
        let dir = home.join(format!("work_{}", name));
        fs::create_dir_all(&dir).unwrap();
        git_cmd(
            &dir,
            &wrapper,
            &["clone", &format!("mgs:{}.git", name), "."],
        );
        git_config_user(&dir);
        git_commit(&dir, "data.txt", &format!("content of {}", name), "init");
        let branch = current_branch(&dir);
        git_cmd(&dir, &wrapper, &["push", "origin", &branch]);
    }

    // Clone each and verify content
    for name in &["repo-alpha", "repo-beta", "repo-gamma"] {
        let dir = home.join(format!("verify_{}", name));
        fs::create_dir_all(&dir).unwrap();
        git_cmd(
            &dir,
            &wrapper,
            &["clone", &format!("mgs:{}.git", name), "."],
        );
        assert_eq!(
            fs::read_to_string(dir.join("data.txt")).unwrap(),
            format!("content of {}", name)
        );
    }
}

#[test]
fn test_re_clone_after_push() {
    let home = test_home();

    let key_path = generate_key(&home, "reclone_user");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &[
            "user",
            "add",
            "reclone_user",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs(
        &home,
        &["repo", "create", "reclone_repo", "--owner", "reclone_user"],
    );

    let wrapper = create_ssh_wrapper(&home, &fp);

    // First clone + push
    let work1 = home.join("work1");
    fs::create_dir_all(&work1).unwrap();
    git_cmd(&work1, &wrapper, &["clone", "mgs:reclone_repo.git", "."]);
    git_config_user(&work1);
    git_commit(&work1, "v1.txt", "version 1", "v1");
    let branch = current_branch(&work1);
    git_cmd(&work1, &wrapper, &["push", "origin", &branch]);

    // Drop work1, re-clone
    let work2 = home.join("work2");
    fs::create_dir_all(&work2).unwrap();
    git_cmd(&work2, &wrapper, &["clone", "mgs:reclone_repo.git", "."]);
    git_config_user(&work2);
    assert_eq!(
        fs::read_to_string(work2.join("v1.txt")).unwrap(),
        "version 1"
    );

    // Push more from work2
    git_commit(&work2, "v2.txt", "version 2", "v2");
    git_cmd(&work2, &wrapper, &["push", "origin", &branch]);

    // Drop work2, re-clone again
    let work3 = home.join("work3");
    fs::create_dir_all(&work3).unwrap();
    git_cmd(&work3, &wrapper, &["clone", "mgs:reclone_repo.git", "."]);
    assert!(work3.join("v1.txt").exists());
    assert!(work3.join("v2.txt").exists());
}
