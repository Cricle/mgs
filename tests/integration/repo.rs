use super::*;

#[test]
fn test_repo_create_and_list() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "repo_key");
    mgs_cmd(
        &home,
        &["user", "add", "owner1", "--key", key_path.to_str().unwrap()],
    );

    let out = mgs_cmd(
        &home,
        &["repo", "create", "team/project", "--owner", "owner1"],
    );
    assert!(out.contains("Created repository 'team/project'"));
    assert!(out.contains("owner: owner1"));

    // Verify the bare repo exists on disk
    assert!(home.join("repos/team/project.git").exists());
    assert!(home.join("repos/team/project.git/HEAD").exists());

    let list_out = mgs_cmd(&home, &["repo", "list"]);
    assert!(list_out.contains("team/project"));
    assert!(list_out.contains("owner: owner1"));
}

#[test]
fn test_repo_remove() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "rr_key");
    mgs_cmd(
        &home,
        &["user", "add", "owner2", "--key", key_path.to_str().unwrap()],
    );
    mgs_cmd(&home, &["repo", "create", "my/repo", "--owner", "owner2"]);

    let out = mgs_cmd(&home, &["repo", "remove", "my/repo"]);
    assert!(out.contains("Removed repository 'my/repo'"));
    assert!(!home.join("repos/my/repo.git").exists());

    let list_out = mgs_cmd(&home, &["repo", "list"]);
    assert!(list_out.contains("No repositories found"));
}

#[test]
fn test_repo_create_nonexistent_owner() {
    let home = mgs_home();
    let err = mgs_cmd_fails(&home, &["repo", "create", "x/y", "--owner", "ghost"]);
    assert!(err.contains("not found"));
}

#[test]
fn test_repo_create_duplicate() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "dup_repo_key");
    mgs_cmd(
        &home,
        &[
            "user",
            "add",
            "dupowner",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs_cmd(&home, &["repo", "create", "duprepo", "--owner", "dupowner"]);
    let err = mgs_cmd_fails(&home, &["repo", "create", "duprepo", "--owner", "dupowner"]);
    assert!(err.contains("already exists"));
}

#[test]
fn test_repo_create_invalid_name() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "inv_key");
    mgs_cmd(
        &home,
        &[
            "user",
            "add",
            "invowner",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    let err = mgs_cmd_fails(
        &home,
        &["repo", "create", "../etc/passwd", "--owner", "invowner"],
    );
    assert!(err.contains("cannot contain '..'"));
}

#[test]
fn test_repo_create_with_dot_git_suffix() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "suffix_key");
    mgs_cmd(
        &home,
        &[
            "user",
            "add",
            "suffixowner",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    // Create with .git suffix — should be stripped
    mgs_cmd(
        &home,
        &["repo", "create", "myproject.git", "--owner", "suffixowner"],
    );

    let list_out = mgs_cmd(&home, &["repo", "list"]);
    assert!(list_out.contains("myproject"));
    // The disk path should not have .git.git
    assert!(home.join("repos/myproject.git").exists());
}

#[test]
fn test_repo_list_empty() {
    let home = mgs_home();
    let out = mgs_cmd(&home, &["repo", "list"]);
    assert!(out.contains("No repositories found"));
}

#[test]
fn test_repo_link_http() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "link_key");
    mgs_cmd(
        &home,
        &[
            "user",
            "add",
            "linkuser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs_cmd(
        &home,
        &["repo", "create", "team/app", "--owner", "linkuser"],
    );

    // Create a temp git repo to link
    let repo_dir = home.join("local_repo");
    fs::create_dir_all(&repo_dir).unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_dir)
        .output()
        .unwrap();

    // Get the token
    let token_out = mgs_cmd(&home, &["user", "token", "show", "linkuser"]);
    let token = token_out.lines().next().unwrap().trim();

    // Run link
    let out = Command::new(env!("CARGO_BIN_EXE_mgs"))
        .env("MGS_HOME", home.to_str().unwrap())
        .current_dir(&repo_dir)
        .args([
            "repo",
            "link",
            "team/app",
            "--user",
            "linkuser",
            "--host",
            "myserver:8080",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "link failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Added remote 'origin'"));
    assert!(stdout.contains(&format!("http://{}@myserver:8080/team/app.git", token)));

    // Verify remote was set
    let remote_out = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(&repo_dir)
        .output()
        .unwrap();
    let remote_url = String::from_utf8_lossy(&remote_out.stdout)
        .trim()
        .to_string();
    assert_eq!(
        remote_url,
        format!("http://{}@myserver:8080/team/app.git", token)
    );
}

#[test]
fn test_repo_link_ssh() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "ssh_link_key");
    mgs_cmd(
        &home,
        &[
            "user",
            "add",
            "sshuser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs_cmd(&home, &["repo", "create", "myrepo", "--owner", "sshuser"]);

    let repo_dir = home.join("local_ssh_repo");
    fs::create_dir_all(&repo_dir).unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_dir)
        .output()
        .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_mgs"))
        .env("MGS_HOME", home.to_str().unwrap())
        .current_dir(&repo_dir)
        .args([
            "repo",
            "link",
            "myrepo",
            "--user",
            "sshuser",
            "--host",
            "myserver:22",
            "--transport",
            "ssh",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ssh://git@myserver:22/myrepo.git"));
}

#[test]
fn test_repo_link_update_existing_remote() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "update_key");
    mgs_cmd(
        &home,
        &[
            "user",
            "add",
            "upduser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs_cmd(&home, &["repo", "create", "updrepo", "--owner", "upduser"]);

    let repo_dir = home.join("local_update_repo");
    fs::create_dir_all(&repo_dir).unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_dir)
        .output()
        .unwrap();
    Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://old.example.com/repo.git",
        ])
        .current_dir(&repo_dir)
        .output()
        .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_mgs"))
        .env("MGS_HOME", home.to_str().unwrap())
        .current_dir(&repo_dir)
        .args([
            "repo",
            "link",
            "updrepo",
            "--user",
            "upduser",
            "--host",
            "newserver:8080",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Updated remote 'origin'"));
}

#[test]
fn test_repo_link_not_git_repo() {
    let home = mgs_home();
    let key_path = generate_test_key(&home, "nogit_key");
    mgs_cmd(
        &home,
        &[
            "user",
            "add",
            "nogituser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs_cmd(
        &home,
        &["repo", "create", "nogitrepo", "--owner", "nogituser"],
    );

    let not_repo_dir = home.join("not_a_repo");
    fs::create_dir_all(&not_repo_dir).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_mgs"))
        .env("MGS_HOME", home.to_str().unwrap())
        .current_dir(&not_repo_dir)
        .args([
            "repo",
            "link",
            "nogitrepo",
            "--user",
            "nogituser",
            "--host",
            "server:8080",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("not a git repository"));
}

#[test]
fn test_repo_remove_nonexistent() {
    let home = mgs_home();
    let out = mgs_cmd(&home, &["repo", "remove", "nope"]);
    assert!(out.contains("not found"));
}
