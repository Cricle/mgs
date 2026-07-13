use mgs::auth::{compute_fingerprint, parse_ssh_public_key};

#[test]
fn test_parse_ed25519_key() {
    let line = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl test@host";
    let (key_type, public_key) = parse_ssh_public_key(line).unwrap();
    assert_eq!(key_type, "ssh-ed25519");
    assert!(public_key.starts_with("AAAAC3NzaC1lZDI1NTE5"));
}

#[test]
fn test_parse_rsa_key() {
    let line = "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQC7FBmMSVTjkMYK6laLr9a/test@host";
    let (key_type, _) = parse_ssh_public_key(line).unwrap();
    assert_eq!(key_type, "ssh-rsa");
}

#[test]
fn test_parse_empty_line() {
    assert!(parse_ssh_public_key("").is_err());
    assert!(parse_ssh_public_key("   ").is_err());
}

#[test]
fn test_parse_comment_line() {
    assert!(parse_ssh_public_key("# this is a comment").is_err());
}

#[test]
fn test_parse_no_comment() {
    assert!(
        parse_ssh_public_key(
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl"
        )
        .is_ok()
    );
}

#[test]
fn test_parse_unsupported_type() {
    assert!(parse_ssh_public_key("ssh-dss AAAAB3NzaC1kc3MAAACBA...").is_err());
}

#[test]
fn test_parse_too_short() {
    assert!(parse_ssh_public_key("ssh-ed25519 short").is_err());
}

#[test]
fn test_parse_no_base64() {
    assert!(parse_ssh_public_key("ssh-ed25519").is_err());
}
