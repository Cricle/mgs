//! MGS — Mini Git Server
//!
//! A lightweight Git server for team-internal use that reuses system SSH
//! for transport and SQLite for metadata storage.
//!
//! # Architecture
//!
//! The server consists of two binaries and a shared library:
//!
//! - **`mgs`** — Administrator CLI for managing users and repositories
//! - **`mgs-ssh`** — SSH forced command entry point, invoked by `sshd`
//!
//! The shared library provides:
//!
//! - [`models`] — Data structures (`User`, `SshKey`, `Repository`)
//! - [`db`] — SQLite database layer with CRUD operations
//! - [`auth`] — SSH key parsing and fingerprint computation
//! - [`git`] — Git repository operations and validation
//! - [`ssh`] — SSH command parsing and routing
//! - [`cli`] — CLI command handlers

pub mod auth;
pub mod cli;
pub mod db;
pub mod git;
pub mod models;
pub mod ssh;

use std::path::PathBuf;

/// Returns the user's home directory.
///
/// Tries `HOME` (Unix), `USERPROFILE` (Windows), then `HOMEDRIVE`+`HOMEPATH` (Windows fallback).
pub fn home_dir() -> anyhow::Result<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        return Ok(PathBuf::from(home));
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        return Ok(PathBuf::from(profile));
    }
    if let (Ok(drive), Ok(path)) = (std::env::var("HOMEDRIVE"), std::env::var("HOMEPATH")) {
        return Ok(PathBuf::from(format!("{}{}", drive, path)));
    }
    anyhow::bail!(
        "could not determine home directory (HOME, USERPROFILE, HOMEDRIVE+HOMEPATH all unset)"
    )
}
