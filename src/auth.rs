//! Authentication.
//!
//! Handles SSH public key parsing and fingerprint computation (via `ssh-keygen`).

use anyhow::{Context, Result, bail};
use std::process::Command;

/// Parses an SSH public key line into `(key_type, public_key_base64)`.
///
/// Expected format: `<type> <base64> [comment]`
///
/// Supported key types: `ssh-ed25519`, `ssh-rsa`, `ecdsa-sha2-nistp256`,
/// `ecdsa-sha2-nistp384`, `ecdsa-sha2-nistp521`.
///
/// Returns an error for empty lines, comments (`#`), unsupported types,
/// or keys shorter than 10 characters.
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

    match key_type.as_str() {
        "ssh-ed25519"
        | "ssh-rsa"
        | "ecdsa-sha2-nistp256"
        | "ecdsa-sha2-nistp384"
        | "ecdsa-sha2-nistp521" => {}
        _ => bail!("unsupported key type: {}", key_type),
    }

    if public_key.len() < 10 {
        bail!("public key too short");
    }

    Ok((key_type, public_key))
}

/// Computes the SHA256 fingerprint of an SSH public key using `ssh-keygen -lf -`.
///
/// Reads the full public key line (type + base64 + optional comment) from stdin.
/// Returns the fingerprint in the format `SHA256:<base64>`.
///
/// Uses a separate thread for stdin writes to avoid pipe buffer deadlock.
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
    let fingerprint = stdout
        .split_whitespace()
        .find(|s| s.starts_with("SHA256:"))
        .context("could not parse fingerprint from ssh-keygen output")?;

    Ok(fingerprint.to_string())
}
