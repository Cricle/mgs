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
- **SSH server** — for SSH transport only (HTTP transport doesn't need SSH)
- **No other dependencies** — MGS bundles SQLite, no external database needed

### Linux / macOS

```bash
# Install git (if not already installed)
sudo apt install git          # Debian/Ubuntu
sudo yum install git          # CentOS/RHEL
brew install git              # macOS

# Verify
git --version
which git-upload-pack
which git-receive-pack
```

### Windows

```powershell
# Install Git for Windows (includes git-upload-pack, git-receive-pack)
# Download from: https://git-scm.com/download/win
# Or with winget:
winget install Git.Git

# Verify (in PowerShell or CMD)
git --version
where git-upload-pack
where git-receive-pack

# SSH server (for SSH transport only, HTTP doesn't need it)
# Windows 10/11 has built-in OpenSSH Server:
# Settings → Apps → Optional Features → Add "OpenSSH Server"
# Or:
Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0
Start-Service sshd
Set-Service -Name sshd -StartupType Automatic
```

## Installation

### From Source

```bash
git clone https://github.com/Cricle/mgs.git
cd mgs
cargo build --release
```

Binaries:
- Linux/macOS: `target/release/mgs`, `target/release/mgs-ssh`
- Windows: `target/release/mgs.exe`, `target/release/mgs-ssh.exe`

### From GitHub Releases

Download the latest binary for your platform from [Releases](https://github.com/Cricle/mgs/releases).

## Quick Start

### 1. Create Users

**Linux / macOS:**

```bash
mgs user add alice --key /home/alice/.ssh/id_ed25519.pub
```

**Windows:**

```powershell
mgs user add alice --key C:\Users\alice\.ssh\id_ed25519.pub
```

Output (same on all platforms):

```
Created user 'alice' with key fingerprint SHA256:xxxxx

HTTP token: a1b2c3d4e5f6...

SSH authorized_keys entry (add to ~/.ssh/authorized_keys on server):
  command="mgs-ssh SHA256:xxxxx",no-port-forwarding,no-X11-forwarding,no-agent-forwarding,no-pty ssh-ed25519 AAAA...

Next steps:
  1. Create a repo:   mgs repo create <name> --owner alice
  2. Clone via HTTP:  git clone http://a1b2c3d4...@<host>:8080/<repo>.git
  3. Clone via SSH:   git clone ssh://git@<host>/<repo>.git
```

### 2. Create Repositories

```bash
mgs repo create team/backend --owner alice
```

Output:

```
Created repository 'team/backend' (owner: alice)

Clone via HTTP: git clone http://a1b2c3d4...@<host>:8080/team/backend.git
Clone via SSH:  git clone ssh://git@<host>/team/backend.git
```

### 3. Configure SSH (for SSH transport)

Copy the `authorized_keys` line from `mgs user add` output to the server.

**Linux / macOS:**

```bash
# Append to authorized_keys
echo 'command="mgs-ssh SHA256:xxxxx",no-port-forwarding,no-X11-forwarding,no-agent-forwarding,no-pty ssh-ed25519 AAAA...' >> ~/.ssh/authorized_keys
chmod 600 ~/.ssh/authorized_keys
```

**Windows (OpenSSH Server):**

```powershell
# For admin users, append to:
# C:\ProgramData\ssh\administrators_authorized_keys
Add-Content -Path "C:\ProgramData\ssh\administrators_authorized_keys" -Value 'command="mgs-ssh.exe SHA256:xxxxx",no-port-forwarding,no-X11-forwarding,no-agent-forwarding,no-pty ssh-ed25519 AAAA...'

# For regular users, append to:
# C:\Users\<username>\.ssh\authorized_keys
Add-Content -Path "C:\Users\alice\.ssh\authorized_keys" -Value 'command="mgs-ssh.exe SHA256:xxxxx",no-port-forwarding,no-X11-forwarding,no-agent-forwarding,no-pty ssh-ed25519 AAAA...'

# Ensure correct permissions
icacls "C:\ProgramData\ssh\administrators_authorized_keys" /inheritance:r /grant "SYSTEM:F" /grant "Administrators:F"
```

**Note:** On Windows, the `command=` must use `mgs-ssh.exe` (with `.exe` extension) and the full path if not in `PATH`:

```
command="C:\path\to\mgs-ssh.exe SHA256:xxxxx",...
```

### 4. Start HTTP Server (for HTTP transport)

```bash
mgs serve --bind 0.0.0.0:8080
```

Output:

```
mgs HTTP server listening on 0.0.0.0:8080

Quick start:
  1. Create user:  mgs user add <name> --key ~/.ssh/id_ed25519.pub
  2. Create repo:  mgs repo create <name> --owner <name>
  3. Clone:        git clone http://<token>@0.0.0.0:8080/<repo>.git
```

**Windows note:** The HTTP server works identically on Windows. No SSH configuration needed for HTTP transport.

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

### HTTPS (Reverse Proxy)

For production, put MGS behind a reverse proxy with TLS.

**Linux (nginx):**

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

**Linux (Caddy):**

```
git.example.com {
    reverse_proxy localhost:8080
}
```

**Windows (IIS):**

```xml
<!-- web.config in IIS site root -->
<configuration>
  <system.webServer>
    <rewrite>
      <rules>
        <rule name="ReverseProxy" stopProcessing="true">
          <match url="(.*)" />
          <action type="Rewrite" url="http://127.0.0.1:8080/{R:1}" />
        </rule>
      </rules>
    </rewrite>
  </system.webServer>
</configuration>
```

Or use [Caddy for Windows](https://caddyserver.com/download) — same config as Linux.

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MGS_HOME` | Data directory path | binary directory |

**Linux / macOS:**

```bash
export MGS_HOME=/var/lib/mgs
# or
mgs --data-dir /var/lib/mgs repo list
```

**Windows:**

```powershell
$env:MGS_HOME = "C:\ProgramData\mgs"
# or
mgs --data-dir C:\ProgramData\mgs repo list
```

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

Cross-compilation for Windows from Linux:

```bash
# Install target
rustup target add x86_64-pc-windows-msvc

# Build (requires Windows SDK / MSVC linker)
cargo build --release --target x86_64-pc-windows-msvc
```

## License

MIT
