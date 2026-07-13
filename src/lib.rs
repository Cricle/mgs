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
pub mod http;
pub mod models;
pub mod ssh;

/// Generates a random 32-byte hex token (64 characters).
pub fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let bytes: [u8; 32] = rng.random();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
