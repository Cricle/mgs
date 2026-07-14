# MGS - Mini Git Server

A lightweight, pure-Rust Git server for team-internal use. Supports both SSH and HTTP transports, stores metadata in SQLite, and provides a CLI for administration.

## Features

- **SSH transport** — leverages existing system `sshd`, no custom SSH implementation needed
- **HTTP transport** — built-in Git Smart HTTP server with token-based authentication
- **SSH public key authentication** — users authenticate with their SSH keys
- **Token authentication** — HTTP access via per-user tokens, auto-generated on user creation
- **SQLite metadata** — single-file database with WAL mode, zero external dependencies
- **CLI management** — `mgs user`, `mgs repo`, `mgs serve`
- **Auto-initialization** — data directory and database created on first use
- **Cross-platform** — Linux, macOS, Windows

## Architecture

```
                    SSH transport                           HTTP transport
                         │                                       │
                         ▼                                       ▼
┌────────────────────────────────┐    ┌────────────────────────────────────┐
│  ~/.ssh/authorized_keys        │    │  mgs serve --bind 0.0.0.0:8080     │
│  command="mgs-ssh SHA256:xxx"  │    │  (axum HTTP server)                │
│         │                      │    │         │                          │
│         ▼                      │    │         ▼                          │
│  mgs-ssh → git-upload-pack     │    │  Token auth → git-upload-pack      │
│           git-receive-pack     │    │             → git-receive-pack     │
│              │                 │    │                │                    │
│              ▼                 │    │                ▼                    │
│         repos/*.git            │    │           repos/*.git               │
│              │                 │    │                │                    │
│              ▼                 │    │                ▼                    │
│          mgs.db (SQLite)       │    │            mgs.db (SQLite)          │
└────────────────────────────────┘    └────────────────────────────────────┘
```

### Components

| Binary | Purpose |
|--------|---------|
| `mgs` | Administrator CLI for managing users, repos, and starting HTTP server |
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

## Prerequisites

- **git** — `git`, `git-upload-pack`, `git-receive-pack` must be in `PATH`
- **SSH server** — for SSH transport (`sshd` with public key auth enabled)
- **No other dependencies** — MGS bundles SQLite, no external database needed

Verify prerequisites:

```bash
git --version
git-upload-pack --version
git-receive-pack --version
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

# Output:
# Created user 'alice' with key fingerprint SHA256:xxxxx
#
# HTTP token: a1b2c3d4e5f6...
#
# SSH authorized_keys entry (add to ~/.ssh/authorized_keys on server):
#   command="mgs-ssh SHA256:xxxxx",no-port-forwarding,no-X11-forwarding,no-agent-forwarding,no-pty ssh-ed25519 AAAA...
#
# Next steps:
#   1. Create a repo:   mgs repo create <name> --owner alice
#   2. Clone via HTTP:  git clone http://a1b2c3d4...@<host>:8080/<repo>.git
#   3. Clone via SSH:   git clone ssh://git@<host>/<repo>.git
```

### 2. Create Repositories

```bash
# Create repo with explicit owner
mgs repo create team/backend --owner alice

# Output:
# Created repository 'team/backend' (owner: alice)
#
# Clone via HTTP: git clone http://a1b2c3d4...@<host>:8080/team/backend.git
# Clone via SSH:  git clone ssh://git@<host>/team/backend.git
```

### 3. Configure SSH (for SSH transport)

Copy the `authorized_keys` line from `mgs user add` output to `~/.ssh/authorized_keys` on the server.

Or manually construct it:

```bash
# Get the fingerprint
ssh-keygen -lf /path/to/key.pub

# Add to authorized_keys
echo 'command="mgs-ssh SHA256:xxxxx",no-port-forwarding,no-X11-forwarding,no-agent-forwarding,no-pty ssh-ed25519 AAAA...' >> ~/.ssh/authorized_keys
```

### 4. Start HTTP Server (for HTTP transport)

```bash
mgs serve --bind 0.0.0.0:8080

# Output:
# mgs HTTP server listening on 0.0.0.0:8080
#
# Quick start:
#   1. Create user:  mgs user add <name> --key ~/.ssh/id_ed25519.pub
#   2. Create repo:  mgs repo create <name> --owner <name>
#   3. Clone:        git clone http://<token>@0.0.0.0:8080/<repo>.git
```

### 5. Use Git

```bash
# Clone via SSH
git clone git@myserver:team/backend.git

# Clone via HTTP (using token)
git clone http://a1b2c3d4...@myserver:8080/team/backend.git

# Push
cd backend
echo "hello" > README.md
git add . && git commit -m "init"
git push origin main

# Fetch / Pull
git pull
```

## CLI Reference

### `mgs user`

Manage users and their SSH keys.

```bash
# Add user with SSH key (auto-generates HTTP token)
mgs user add <username> --key <pubkey_file>

# List all users (shows token hint)
mgs user list

# Remove user (cascades to keys)
mgs user remove <username>

# Show user's HTTP token
mgs user token show <username>

# Regenerate user's HTTP token
mgs user token regenerate <username>

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

### `mgs serve`

Start the HTTP server for Git Smart HTTP protocol.

```bash
# Start with default settings (0.0.0.0:8080)
mgs serve

# Bind to custom address
mgs serve --bind 127.0.0.1:9000
```

## HTTP Transport

### Authentication

HTTP requests use token-based authentication via HTTP Basic Auth:

- **Format 1**: `http://<token>@host/repo.git` — token as username, empty password
- **Format 2**: `http://user:<token>@host/repo.git` — token as password

Tokens are 64-character hex strings, auto-generated when creating a user.

### Git Smart HTTP Protocol

The HTTP server implements the [Git Smart HTTP Protocol](https://git-scm.com/docs/http-protocol):

| Method | Path | Description |
|--------|------|-------------|
| GET | `/<repo>/info/refs?service=git-upload-pack` | Advertise refs (clone/fetch) |
| POST | `/<repo>/git-upload-pack` | Pack data for clone/fetch |
| GET | `/<repo>/info/refs?service=git-receive-pack` | Advertise refs (push) |
| POST | `/<repo>/git-receive-pack` | Pack data for push |

### HTTPS

For production, put MGS behind a reverse proxy (nginx, caddy) with TLS:

```nginx
server {
    listen 443 ssl;
    server_name git.example.com;

    ssl_certificate     /etc/ssl/cert.pem;
    ssl_certificate_key /etc/ssl/key.pem;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MGS_HOME` | Data directory path | binary directory |

## Database Schema

```sql
-- Users (with HTTP token)
CREATE TABLE users (
    id          INTEGER PRIMARY KEY,
    username    TEXT NOT NULL UNIQUE,
    token       TEXT UNIQUE,
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

## Security

- **Repository names**: only `[a-zA-Z0-9/_.-]` allowed, rejects `..` (path traversal)
- **Usernames**: only `[a-zA-Z0-9_-]` allowed
- **SSH**: uses `no-port-forwarding,no-X11-forwarding,no-agent-forwarding,no-pty` restrictions
- **HTTP**: token transmitted via Basic Auth (use HTTPS in production)
- **Git commands**: verified at startup (`git`, `git-upload-pack`, `git-receive-pack`)

## Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests (163 tests: unit + integration + E2E)
cargo test

# Check formatting and lints
cargo fmt --check
cargo clippy -- -D warnings
```

## License

MIT
