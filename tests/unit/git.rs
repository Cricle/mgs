use mgs::git::{
    init_bare_repo, normalize_repo_name, repo_disk_path, validate_repo_name, validate_username,
};
use std::path::PathBuf;
use tempfile::TempDir;

// --- validate_repo_name ---

#[test]
fn test_validate_repo_name_valid() {
    assert!(validate_repo_name("myrepo").is_ok());
    assert!(validate_repo_name("team/project").is_ok());
    assert!(validate_repo_name("my-repo_v2").is_ok());
    assert!(validate_repo_name("a.b/c-d_e").is_ok());
}

#[test]
fn test_validate_repo_name_empty() {
    assert!(validate_repo_name("").is_err());
}

#[test]
fn test_validate_repo_name_traversal() {
    assert!(validate_repo_name("../etc/passwd").is_err());
    assert!(validate_repo_name("foo/../bar").is_err());
    assert!(validate_repo_name("foo/..").is_err());
}

#[test]
fn test_validate_repo_name_invalid_chars() {
    assert!(validate_repo_name("my repo").is_err());
    assert!(validate_repo_name("my@repo").is_err());
    assert!(validate_repo_name("repo!").is_err());
    assert!(validate_repo_name("a b").is_err());
}

// --- normalize_repo_name ---

#[test]
fn test_normalize_strips_git() {
    assert_eq!(normalize_repo_name("myrepo.git"), "myrepo");
    assert_eq!(normalize_repo_name("team/project.git"), "team/project");
}

#[test]
fn test_normalize_no_git() {
    assert_eq!(normalize_repo_name("myrepo"), "myrepo");
    assert_eq!(normalize_repo_name("team/project"), "team/project");
}

// --- validate_username ---

#[test]
fn test_validate_username_valid() {
    assert!(validate_username("alice").is_ok());
    assert!(validate_username("user_1").is_ok());
    assert!(validate_username("my-user").is_ok());
    assert!(validate_username("A1_b2-C3").is_ok());
}

#[test]
fn test_validate_username_empty() {
    assert!(validate_username("").is_err());
}

#[test]
fn test_validate_username_invalid_chars() {
    assert!(validate_username("my user").is_err());
    assert!(validate_username("user@host").is_err());
    assert!(validate_username("a.b").is_err());
    assert!(validate_username("a/b").is_err());
}

// --- repo_disk_path ---

#[test]
fn test_repo_disk_path_appends_git() {
    let data = PathBuf::from("/data");
    let path = repo_disk_path(&data, "myrepo");
    assert_eq!(path, PathBuf::from("/data/repos/myrepo.git"));
}

#[test]
fn test_repo_disk_path_nested() {
    let data = PathBuf::from("/data");
    let path = repo_disk_path(&data, "team/project");
    assert_eq!(path, PathBuf::from("/data/repos/team/project.git"));
}

// --- init_bare_repo ---

#[test]
fn test_init_bare_repo_creates_directory() {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path().join("test.git");
    init_bare_repo(&repo_path).unwrap();
    assert!(repo_path.exists());
    assert!(repo_path.join("HEAD").exists());
    assert!(repo_path.join("objects").exists());
}

#[test]
fn test_init_bare_repo_fails_if_exists() {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path().join("test.git");
    init_bare_repo(&repo_path).unwrap();
    assert!(init_bare_repo(&repo_path).is_err());
}
