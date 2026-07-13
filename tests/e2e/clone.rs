use super::*;
use std::fs;

#[test]
fn test_clone_empty_repo() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "user1");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &["user", "add", "user1", "--key", key_path.to_str().unwrap()],
    );
    mgs(&home, &["repo", "create", "myrepo", "--owner", "user1"]);

    let wrapper = create_ssh_wrapper(&home, &fp);
    let clone_dir = home.join("clone");
    fs::create_dir_all(&clone_dir).unwrap();

    let out = git_cmd(&clone_dir, &wrapper, &["clone", "mgs:myrepo.git", "."]);
    assert!(
        out.contains("Cloning into")
            || out.contains("warning: You appear to have cloned an empty repository")
    );
    assert!(clone_dir.join(".git").exists());
}

#[test]
fn test_clone_with_dot_git_suffix() {
    let home = test_home();
    mgs(&home, &["init"]);

    let key_path = generate_key(&home, "suffix_user");
    let fp = key_fingerprint(&key_path);
    mgs(
        &home,
        &[
            "user",
            "add",
            "suffix_user",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    // Create without .git
    mgs(
        &home,
        &["repo", "create", "myproject", "--owner", "suffix_user"],
    );

    let wrapper = create_ssh_wrapper(&home, &fp);
    let work = home.join("suffix_work");
    fs::create_dir_all(&work).unwrap();

    // Clone WITH .git suffix — should still work
    git_cmd(&work, &wrapper, &["clone", "mgs:myproject.git", "."]);
    git_config_user(&work);
    git_commit(&work, "readme.md", "# My Project", "init");
    let branch = current_branch(&work);
    git_cmd(&work, &wrapper, &["push", "origin", &branch]);

    // Clone WITHOUT .git suffix — should also work
    let work2 = home.join("suffix_work2");
    fs::create_dir_all(&work2).unwrap();
    git_cmd(&work2, &wrapper, &["clone", "mgs:myproject", "."]);
    assert_eq!(
        fs::read_to_string(work2.join("readme.md")).unwrap(),
        "# My Project"
    );
}
