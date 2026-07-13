use anyhow::{Context, Result, bail};
use std::process::Command;

use crate::db::Database;
use crate::models::PermLevel;

/// Parse an SSH public key file line (e.g. "ssh-ed25519 AAAA... comment").
/// Returns (key_type, public_key_base64).
pub fn parse_ssh_public_key(line: &str) -> Result<(String, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        bail!("empty or comment line");
    }
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        bail!("invalid public key format, expected: <type> <base64> [comment]");
    }
    let key_type = parts[0].to_string();
    let public_key = parts[1].to_string();

    // Validate key type
    match key_type.as_str() {
        "ssh-ed25519"
        | "ssh-rsa"
        | "ecdsa-sha2-nistp256"
        | "ecdsa-sha2-nistp384"
        | "ecdsa-sha2-nistp521" => {}
        _ => bail!("unsupported key type: {}", key_type),
    }

    // Basic base64 length check
    if public_key.len() < 10 {
        bail!("public key too short");
    }

    Ok((key_type, public_key))
}

/// Compute SHA256 fingerprint of an SSH public key using ssh-keygen.
pub fn compute_fingerprint(public_key_line: &str) -> Result<String> {
    let mut child = Command::new("ssh-keygen")
        .args(["-lf", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to spawn ssh-keygen")?;

    use std::io::Write;
    let mut stdin = child
        .stdin
        .take()
        .context("failed to open ssh-keygen stdin")?;
    let data = public_key_line.to_owned();
    // Write stdin in a separate thread to avoid potential deadlock
    let handle = std::thread::spawn(move || {
        let _ = stdin.write_all(data.as_bytes());
    });

    let result = child.wait_with_output()?;
    handle.join().ok();

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        bail!("ssh-keygen failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&result.stdout);
    // Output format: "256 SHA256:xxxx comment (ED25519)"
    let fingerprint = stdout
        .split_whitespace()
        .find(|s| s.starts_with("SHA256:"))
        .context("could not parse fingerprint from ssh-keygen output")?;

    Ok(fingerprint.to_string())
}

/// Check if a user has at least `required` permission on a repo.
/// Returns Ok(()) if allowed, Err if denied.
pub fn check_permission(
    db: &Database,
    user_id: i64,
    repo_id: i64,
    required: &PermLevel,
) -> Result<()> {
    let effective = db
        .get_permission(user_id, repo_id)?
        .with_context(|| "access denied")?;

    if effective.satisfies(required) {
        Ok(())
    } else {
        bail!(
            "permission denied: need {}, have {}",
            required.as_str(),
            effective.as_str()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
