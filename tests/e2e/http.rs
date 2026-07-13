use super::*;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

/// Starts the mgs HTTP server in the background and returns the child process and port.
fn start_http_server(home: &Path, port: u16) -> (Child, String) {
    let mgs_bin = env!("CARGO_BIN_EXE_mgs");
    let bind = format!("127.0.0.1:{}", port);
    let child = Command::new(mgs_bin)
        .env("MGS_HOME", home.to_str().unwrap())
        .args(["serve", "--bind", &bind])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start mgs serve");

    // Wait for server to start
    thread::sleep(Duration::from_millis(500));

    (child, bind)
}

/// Extracts the HTTP token from `mgs user add` output.
fn extract_token(output: &str) -> String {
    output
        .lines()
        .find(|l| l.starts_with("HTTP token:"))
        .expect("no token in output")
        .strip_prefix("HTTP token: ")
        .unwrap()
        .trim()
        .to_string()
}

#[test]
fn test_http_clone_and_push() {
    let home = test_home();
    let port = 19080 + (TEST_COUNTER.load(Ordering::SeqCst) as u16);

    // Create user and get token
    let key_path = generate_key(&home, "http_key");
    let add_out = mgs(
        &home,
        &[
            "user",
            "add",
            "httpuser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    let token = extract_token(&add_out);

    // Create repo
    mgs(
        &home,
        &["repo", "create", "testrepo", "--owner", "httpuser"],
    );

    // Start HTTP server
    let (mut server, bind) = start_http_server(&home, port);

    // Clone via HTTP
    let clone_dir = home.join("clone_workdir");
    fs::create_dir_all(&clone_dir).unwrap();
    let clone_url = format!("http://{}@{}/testrepo.git", token, bind);

    let output = Command::new("git")
        .current_dir(&home)
        .args(["clone", &clone_url, clone_dir.to_str().unwrap()])
        .output()
        .expect("failed to run git clone");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Kill server
    let _ = server.kill();

    assert!(
        output.status.success(),
        "git clone failed:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // Verify clone created a git repo
    assert!(clone_dir.join(".git").exists());

    // Make a commit and push
    git_config_user(&clone_dir);
    git_commit(&clone_dir, "README.md", "hello from HTTP", "initial commit");

    // Get the current branch name
    let branch = current_branch(&clone_dir);

    // Start server again for push
    let (mut server2, bind2) = start_http_server(&home, port);

    let push_url = format!("http://{}@{}/testrepo.git", token, bind2);
    let push_output = Command::new("git")
        .current_dir(&clone_dir)
        .args(["push", &push_url, &branch])
        .output()
        .expect("failed to run git push");

    let _ = server2.kill();

    let push_stdout = String::from_utf8_lossy(&push_output.stdout).to_string();
    let push_stderr = String::from_utf8_lossy(&push_output.stderr).to_string();

    assert!(
        push_output.status.success(),
        "git push failed:\nstdout: {}\nstderr: {}",
        push_stdout,
        push_stderr
    );
}

#[test]
fn test_http_clone_unauthorized() {
    let home = test_home();
    let port = 19090 + (TEST_COUNTER.load(Ordering::SeqCst) as u16);

    // Create user and repo
    let key_path = generate_key(&home, "auth_key");
    mgs(
        &home,
        &[
            "user",
            "add",
            "authuser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs(
        &home,
        &["repo", "create", "authrepo", "--owner", "authuser"],
    );

    // Start HTTP server
    let (mut server, bind) = start_http_server(&home, port);

    // Try to clone with wrong token
    let clone_url = format!("http://wrongtoken@{}/authrepo.git", bind);
    let output = Command::new("git")
        .current_dir(&home)
        .args([
            "clone",
            &clone_url,
            home.join("bad_clone").to_str().unwrap(),
        ])
        .output()
        .expect("failed to run git clone");

    let _ = server.kill();

    assert!(
        !output.status.success(),
        "clone should have failed with bad token"
    );
}

#[test]
fn test_http_clone_nonexistent_repo() {
    let home = test_home();
    let port = 19100 + (TEST_COUNTER.load(Ordering::SeqCst) as u16);

    let key_path = generate_key(&home, "nonexist_key");
    let add_out = mgs(
        &home,
        &[
            "user",
            "add",
            "nonexistuser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    let token = extract_token(&add_out);

    // Don't create any repo
    let (mut server, bind) = start_http_server(&home, port);

    let clone_url = format!("http://{}@{}/nonexist.git", token, bind);
    let output = Command::new("git")
        .current_dir(&home)
        .args([
            "clone",
            &clone_url,
            home.join("bad_clone").to_str().unwrap(),
        ])
        .output()
        .expect("failed to run git clone");

    let _ = server.kill();

    assert!(
        !output.status.success(),
        "clone should have failed for nonexistent repo"
    );
}

#[test]
fn test_http_fetch_after_push() {
    let home = test_home();
    let port = 19110 + (TEST_COUNTER.load(Ordering::SeqCst) as u16);

    // Setup
    let key_path = generate_key(&home, "fetch_key");
    let add_out = mgs(
        &home,
        &[
            "user",
            "add",
            "fetchuser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    let token = extract_token(&add_out);

    mgs(
        &home,
        &["repo", "create", "fetchrepo", "--owner", "fetchuser"],
    );

    // Clone
    let (mut server, bind) = start_http_server(&home, port);
    let clone_url = format!("http://{}@{}/fetchrepo.git", token, bind);
    let clone_dir = home.join("clone1");
    fs::create_dir_all(&clone_dir).unwrap();

    let output = Command::new("git")
        .current_dir(&home)
        .args(["clone", &clone_url, clone_dir.to_str().unwrap()])
        .output()
        .expect("failed to clone");
    assert!(output.status.success());
    let _ = server.kill();

    // Make a commit and push
    git_config_user(&clone_dir);
    git_commit(&clone_dir, "file1.txt", "content1", "first commit");
    let branch = current_branch(&clone_dir);

    let (mut server2, bind2) = start_http_server(&home, port);
    let push_url = format!("http://{}@{}/fetchrepo.git", token, bind2);
    let push_output = Command::new("git")
        .current_dir(&clone_dir)
        .args(["push", &push_url, &branch])
        .output()
        .expect("failed to push");
    assert!(push_output.status.success());
    let _ = server2.kill();

    // Fetch from another clone
    let (mut server3, bind3) = start_http_server(&home, port);
    let fetch_url = format!("http://{}@{}/fetchrepo.git", token, bind3);
    let clone_dir2 = home.join("clone2");

    let output = Command::new("git")
        .current_dir(&home)
        .args(["clone", &fetch_url, clone_dir2.to_str().unwrap()])
        .output()
        .expect("failed to clone");
    let _ = server3.kill();

    assert!(output.status.success());
    assert!(clone_dir2.join("file1.txt").exists());
}

#[test]
fn test_http_multiple_repos() {
    let home = test_home();
    let port = 19120 + (TEST_COUNTER.load(Ordering::SeqCst) as u16);

    let key_path = generate_key(&home, "multi_key");
    let add_out = mgs(
        &home,
        &[
            "user",
            "add",
            "multiuser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    let token = extract_token(&add_out);

    // Create multiple repos
    mgs(&home, &["repo", "create", "repo_a", "--owner", "multiuser"]);
    mgs(&home, &["repo", "create", "repo_b", "--owner", "multiuser"]);

    let (mut server, bind) = start_http_server(&home, port);

    // Clone both repos
    let url_a = format!("http://{}@{}/repo_a.git", token, bind);
    let url_b = format!("http://{}@{}/repo_b.git", token, bind);

    let dir_a = home.join("work_a");
    let dir_b = home.join("work_b");
    fs::create_dir_all(&dir_a).unwrap();
    fs::create_dir_all(&dir_b).unwrap();

    let out_a = Command::new("git")
        .current_dir(&home)
        .args(["clone", &url_a, dir_a.to_str().unwrap()])
        .output()
        .unwrap();
    let out_b = Command::new("git")
        .current_dir(&home)
        .args(["clone", &url_b, dir_b.to_str().unwrap()])
        .output()
        .unwrap();

    let _ = server.kill();

    assert!(out_a.status.success(), "clone repo_a failed");
    assert!(out_b.status.success(), "clone repo_b failed");
    assert!(dir_a.join(".git").exists());
    assert!(dir_b.join(".git").exists());
}

#[test]
fn test_http_nested_repo() {
    let home = test_home();
    let port = 19130 + (TEST_COUNTER.load(Ordering::SeqCst) as u16);

    let key_path = generate_key(&home, "nested_key");
    let add_out = mgs(
        &home,
        &[
            "user",
            "add",
            "nesteduser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    let token = extract_token(&add_out);

    mgs(
        &home,
        &["repo", "create", "team/backend", "--owner", "nesteduser"],
    );

    let (mut server, bind) = start_http_server(&home, port);
    let clone_url = format!("http://{}@{}/team/backend.git", token, bind);
    let clone_dir = home.join("nested_work");
    fs::create_dir_all(&clone_dir).unwrap();

    let output = Command::new("git")
        .current_dir(&home)
        .args(["clone", &clone_url, clone_dir.to_str().unwrap()])
        .output()
        .expect("failed to clone");

    let _ = server.kill();

    assert!(
        output.status.success(),
        "clone nested repo failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(clone_dir.join(".git").exists());
}

#[test]
fn test_http_multiple_users_same_repo() {
    let home = test_home();
    let port = 19140 + (TEST_COUNTER.load(Ordering::SeqCst) as u16);

    // Create two users
    let key1 = generate_key(&home, "user1_key");
    let add_out1 = mgs(
        &home,
        &["user", "add", "user1", "--key", key1.to_str().unwrap()],
    );
    let token1 = extract_token(&add_out1);

    let key2 = generate_key(&home, "user2_key");
    let add_out2 = mgs(
        &home,
        &["user", "add", "user2", "--key", key2.to_str().unwrap()],
    );
    let token2 = extract_token(&add_out2);

    // Create repo owned by user1
    mgs(&home, &["repo", "create", "shared", "--owner", "user1"]);

    let (mut server, bind) = start_http_server(&home, port);

    // Both users should be able to clone
    let url1 = format!("http://{}@{}/shared.git", token1, bind);
    let url2 = format!("http://{}@{}/shared.git", token2, bind);

    let dir1 = home.join("user1_work");
    let dir2 = home.join("user2_work");
    fs::create_dir_all(&dir1).unwrap();
    fs::create_dir_all(&dir2).unwrap();

    let out1 = Command::new("git")
        .current_dir(&home)
        .args(["clone", &url1, dir1.to_str().unwrap()])
        .output()
        .unwrap();
    let out2 = Command::new("git")
        .current_dir(&home)
        .args(["clone", &url2, dir2.to_str().unwrap()])
        .output()
        .unwrap();

    let _ = server.kill();

    assert!(out1.status.success(), "user1 clone failed");
    assert!(out2.status.success(), "user2 clone failed");
}

#[test]
fn test_http_push_and_re_clone() {
    let home = test_home();
    let port = 19150 + (TEST_COUNTER.load(Ordering::SeqCst) as u16);

    let key_path = generate_key(&home, "reclone_key");
    let add_out = mgs(
        &home,
        &[
            "user",
            "add",
            "recloneuser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    let token = extract_token(&add_out);

    mgs(
        &home,
        &["repo", "create", "reclone", "--owner", "recloneuser"],
    );

    // Clone, commit, push
    let (mut server, bind) = start_http_server(&home, port);
    let clone_url = format!("http://{}@{}/reclone.git", token, bind);
    let work_dir = home.join("work");
    fs::create_dir_all(&work_dir).unwrap();

    Command::new("git")
        .current_dir(&home)
        .args(["clone", &clone_url, work_dir.to_str().unwrap()])
        .output()
        .unwrap();
    let _ = server.kill();

    git_config_user(&work_dir);
    git_commit(&work_dir, "data.txt", "hello world", "add data");
    let branch = current_branch(&work_dir);

    let (mut server2, bind2) = start_http_server(&home, port);
    let push_url = format!("http://{}@{}/reclone.git", token, bind2);
    Command::new("git")
        .current_dir(&work_dir)
        .args(["push", &push_url, &branch])
        .output()
        .unwrap();
    let _ = server2.kill();

    // Re-clone and verify data persisted
    let (mut server3, bind3) = start_http_server(&home, port);
    let reclone_url = format!("http://{}@{}/reclone.git", token, bind3);
    let reclone_dir = home.join("reclone_work");
    fs::create_dir_all(&reclone_dir).unwrap();

    let output = Command::new("git")
        .current_dir(&home)
        .args(["clone", &reclone_url, reclone_dir.to_str().unwrap()])
        .output()
        .unwrap();
    let _ = server3.kill();

    assert!(output.status.success());
    let content = fs::read_to_string(reclone_dir.join("data.txt")).unwrap();
    assert_eq!(content, "hello world");
}

#[test]
fn test_http_no_auth_header() {
    let home = test_home();
    let port = 19160 + (TEST_COUNTER.load(Ordering::SeqCst) as u16);

    let key_path = generate_key(&home, "noauth_key");
    mgs(
        &home,
        &[
            "user",
            "add",
            "noauthuser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    mgs(
        &home,
        &["repo", "create", "noauthrepo", "--owner", "noauthuser"],
    );

    let (mut server, bind) = start_http_server(&home, port);

    // Clone without any credentials in URL
    let clone_url = format!("http://{}/noauthrepo.git", bind);
    let output = Command::new("git")
        .current_dir(&home)
        .args([
            "clone",
            &clone_url,
            home.join("noauth_work").to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let _ = server.kill();

    // Should fail because no auth provided
    assert!(!output.status.success(), "clone without auth should fail");
}

#[test]
fn test_http_multiple_commits_push() {
    let home = test_home();
    let port = 19170 + (TEST_COUNTER.load(Ordering::SeqCst) as u16);

    let key_path = generate_key(&home, "multicommit_key");
    let add_out = mgs(
        &home,
        &[
            "user",
            "add",
            "multicommituser",
            "--key",
            key_path.to_str().unwrap(),
        ],
    );
    let token = extract_token(&add_out);

    mgs(
        &home,
        &[
            "repo",
            "create",
            "multicommit",
            "--owner",
            "multicommituser",
        ],
    );

    // Clone
    let (mut server, bind) = start_http_server(&home, port);
    let clone_url = format!("http://{}@{}/multicommit.git", token, bind);
    let work_dir = home.join("work");
    fs::create_dir_all(&work_dir).unwrap();

    Command::new("git")
        .current_dir(&home)
        .args(["clone", &clone_url, work_dir.to_str().unwrap()])
        .output()
        .unwrap();
    let _ = server.kill();

    git_config_user(&work_dir);

    // Make multiple commits
    for i in 0..5 {
        git_commit(
            &work_dir,
            &format!("file{}.txt", i),
            &format!("content {}", i),
            &format!("commit {}", i),
        );
    }

    let branch = current_branch(&work_dir);

    // Push all commits
    let (mut server2, bind2) = start_http_server(&home, port);
    let push_url = format!("http://{}@{}/multicommit.git", token, bind2);
    let push_output = Command::new("git")
        .current_dir(&work_dir)
        .args(["push", &push_url, &branch])
        .output()
        .unwrap();
    let _ = server2.kill();

    assert!(
        push_output.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push_output.stderr)
    );

    // Clone again and verify all files
    let (mut server3, bind3) = start_http_server(&home, port);
    let reclone_url = format!("http://{}@{}/multicommit.git", token, bind3);
    let verify_dir = home.join("verify");
    fs::create_dir_all(&verify_dir).unwrap();

    Command::new("git")
        .current_dir(&home)
        .args(["clone", &reclone_url, verify_dir.to_str().unwrap()])
        .output()
        .unwrap();
    let _ = server3.kill();

    for i in 0..5 {
        let path = verify_dir.join(format!("file{}.txt", i));
        assert!(path.exists(), "file{}.txt should exist", i);
        assert_eq!(fs::read_to_string(&path).unwrap(), format!("content {}", i));
    }
}
