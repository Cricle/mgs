use mgs::ssh::{GitCommand, parse_command};

#[test]
fn test_parse_upload_pack_with_quotes() {
    let (cmd, repo) = parse_command("git-upload-pack 'myrepo.git'").unwrap();
    assert!(matches!(cmd, GitCommand::UploadPack));
    assert_eq!(repo, "myrepo");
}

#[test]
fn test_parse_receive_pack_with_quotes() {
    let (cmd, repo) = parse_command("git-receive-pack 'myrepo'").unwrap();
    assert!(matches!(cmd, GitCommand::ReceivePack));
    assert_eq!(repo, "myrepo");
}

#[test]
fn test_parse_strips_dot_git() {
    let (_, repo) = parse_command("git-upload-pack 'project.git'").unwrap();
    assert_eq!(repo, "project");
}

#[test]
fn test_parse_no_dot_git() {
    let (_, repo) = parse_command("git-upload-pack 'project'").unwrap();
    assert_eq!(repo, "project");
}

#[test]
fn test_parse_double_quotes() {
    let (cmd, repo) = parse_command("git-upload-pack \"myrepo.git\"").unwrap();
    assert!(matches!(cmd, GitCommand::UploadPack));
    assert_eq!(repo, "myrepo");
}

#[test]
fn test_parse_no_quotes() {
    let (cmd, repo) = parse_command("git-upload-pack myrepo").unwrap();
    assert!(matches!(cmd, GitCommand::UploadPack));
    assert_eq!(repo, "myrepo");
}

#[test]
fn test_parse_unsupported_command() {
    assert!(parse_command("git-repo-list 'repo'").is_err());
}

#[test]
fn test_parse_empty() {
    assert!(parse_command("").is_err());
}

#[test]
fn test_parse_too_many_parts() {
    assert!(parse_command("git-upload-pack repo extra").is_err());
}

#[test]
fn test_parse_with_whitespace() {
    let (_, repo) = parse_command("  git-upload-pack 'myrepo'  ").unwrap();
    assert_eq!(repo, "myrepo");
}
