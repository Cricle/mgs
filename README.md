# MGS - Mini Git Server

A lightweight, pure-Rust Git server for team-internal use. Reuses system SSH for transport, stores metadata in SQLite, and provides a CLI for administration.

## Features

- **SSH transport** — leverages existing system `sshd`, no custom SSH implementation needed
- **SSH public key authentication** — users authenticate with their SSH keys
- **SQLite metadata** — single-file database with WAL mode, zero external dependencies
- **CLI management** — `mgs user`, `mgs repo`
- **Auto-initialization** — data directory and database created on first use
- **Cross-platform** — Linux, macOS, Windows

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  $ git clone git@myserver:team/project.git               │
└──────────────────────┬──────────────────────────────────┘
                       │ SSH (system sshd)
                       ▼
┌─────────────────────────────────────────────────────────┐
│  ~/.ssh/authorized_keys                                  │
│  command="mgs-ssh SHA256:xxxx",no-pty,... ssh-ed25519 AAAA│
│       │                                                  │
│       ▼                                                  │
│  mgs-ssh → mgs-core (git protocol) → repos/              │
│                    ↓                                     │
│              mgs.db (SQLite)                              │
└─────────────────────────────────────────────────────────┘
```

### Components

| Binary | Purpose |
|--------|---------|
| `mgs` | Administrator CLI for managing users and repos |
| `mgs-ssh` | SSH forced command entry point, called by `sshd` |

### Data Directory

Default: the directory containing the `mgs` binary (override with `MGS_HOME` env var or `--data-dir` flag)

```
<data_dir>/
├── mgs.db          # SQLite database
└── repos/
    ├── team/
    │   └── project.git/
    └── personal/
        └── alice/
            └── scratch.git/
```

## Installation

### From Source

```bash
git clone https://github.com/Cricle/mgs.git
cd mgs
cargo build --release
```

Binaries will be in `target/release/mgs` and `target/release/mgs-ssh`.

### From GitHub Releases

Download the latest binary for your platform from [Releases](https://github.com/Cricle/mgs/releases).

## Quick Start

### 1. Create Users

```bash
# Add user with their SSH public key
mgs user add alice --key /home/alice/.ssh/id_ed25519.pub
mgs user add bob --key /home/bob/.ssh/id_rsa.pub

# List users
mgs user list

# Add additional SSH key for a user
mgs user key add alice --key /home/alice/.ssh/id_rsa.pub

# List user's keys
mgs user key list alice
```

### 2. Create Repositories

```bash
# Create repo with explicit owner
mgs repo create team/backend --owner alice

# Create repo (owner defaults to current system user)
mgs repo create team/frontend

# List repositories
mgs repo list
```

### 3. Configure SSH

For each user, add their public key to `~/.ssh/authorized_keys` on the server:

```bash
command="mgs-ssh SHA256:xxxxx",no-pty,no-port-forwarding,no-X11-forwarding,no-agent-forwarding ssh-ed25519 AAAA...
```

You can get the fingerprint with:

```bash
ssh-keygen -lf /path/to/key.pub
```

### 4. Use Git

```bash
# Clone
git clone git@myserver:team/backend.git

# Push
cd backend
git push origin main

# Fetch / Pull
git pull
```

## CLI Reference

### `mgs user`

Manage users and their SSH keys.

```bash
# Add user with SSH key
mgs user add <username> --key <pubkey_file>

# List all users
mgs user list

# Remove user (cascades to keys and permissions)
mgs user remove <username>

# Add SSH key to existing user
mgs user key add <username> --key <pubkey_file>

# List user's SSH keys
mgs user key list <username>

# Remove SSH key by fingerprint
mgs user key remove <fingerprint>
```

### `mgs repo`

Manage Git repositories.

```bash
# Create repository
mgs repo create <name> [--owner <username>]

# List all repositories
mgs repo list

# Remove repository (deletes disk files and DB record)
mgs repo remove <name>
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MGS_HOME` | Data directory path | binary directory |

## Database Schema

```sql
-- Users
CREATE TABLE users (
    id          INTEGER PRIMARY KEY,
    username    TEXT NOT NULL UNIQUE,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- SSH public keys (one user can have multiple)
CREATE TABLE ssh_keys (
    id          INTEGER PRIMARY KEY,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_type    TEXT NOT NULL,
    public_key  TEXT NOT NULL UNIQUE,
    fingerprint TEXT NOT NULL UNIQUE,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Repositories
CREATE TABLE repositories (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    owner_id    INTEGER NOT NULL REFERENCES users(id),
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

```

## Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Check formatting and lints
cargo fmt --check
cargo clippy -- -D warnings
```

## License

MIT
