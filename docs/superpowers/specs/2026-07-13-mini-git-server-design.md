# MGS - Mini Git Server 设计文档

## 概述

纯 Rust 实现的团队内部 Git 服务器，复用系统 SSH，SQLite 存储元数据，CLI 管理。

## 需求

| 维度 | 选择 |
|------|------|
| 用途 | 团队内部 |
| 传输 | SSH（复用系统 sshd） |
| 认证 | SSH 公钥 |
| 存储 | SQLite |
| 管理 | CLI 命令 |
| Git 功能 | clone/push/pull、分支/标签管理、权限控制 |
| Web UI | 无 |

## 架构

```
┌─────────────────────────────────────────────────────────┐
│  用户机器                                                │
│  $ git clone git@myserver:team/project.git               │
└──────────────────────┬──────────────────────────────────┘
                       │ SSH (系统 sshd)
                       ▼
┌─────────────────────────────────────────────────────────┐
│  服务器 (myserver)                                       │
│                                                          │
│  ~/.ssh/authorized_keys                                  │
│  command="mgs-ssh SHA256:xxxx",no-pty,no-port-forwarding  │
│  ,no-X11-forwarding,no-agent-forwarding ssh-ed25519 AAAA │
│       │                                                  │
│       ▼                                                  │
│  ┌─────────┐    ┌───────────┐    ┌──────────────────┐   │
│  │ mgs-ssh │───▶│  mgs-core │───▶│ repos/            │   │
│  │ (入口)   │    │ (git协议)  │    │  team/project.git │   │
│  └─────────┘    └─────┬─────┘    └──────────────────┘   │
│                       │                                  │
│                       ▼                                  │
│                 ┌───────────┐                            │
│                 │  mgs.db   │                            │
│                 │  (SQLite) │                            │
│                 └───────────┘                            │
│                                                          │
│  管理员:                                                 │
│  $ mgs user add alice --key ~/.ssh/alice.pub             │
│  $ mgs repo create team/project                          │
│  $ mgs acl grant alice team/project --perm push          │
└─────────────────────────────────────────────────────────┘
```

### 组件

| 组件 | 作用 |
|------|------|
| `mgs` | 管理员 CLI（用户/仓库/权限管理） |
| `mgs-ssh` | SSH forced command 入口，被 sshd 调用 |
| `mgs-core` | 共享库，包含 git pack 协议、数据库、权限检查 |

### 数据目录

```
/var/lib/mgs/          # 或 ~/.mgs/ 用于开发
├── mgs.db             # SQLite 数据库
└── repos/
    ├── team/
    │   └── project.git/
    └── personal/
        └── alice/
            └── scratch.git/
```

## 数据库

SQLite，单文件，WAL 模式。

### 表结构

```sql
CREATE TABLE users (
    id          INTEGER PRIMARY KEY,
    username    TEXT NOT NULL UNIQUE,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE ssh_keys (
    id          INTEGER PRIMARY KEY,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_type    TEXT NOT NULL,
    public_key  TEXT NOT NULL UNIQUE,
    fingerprint TEXT NOT NULL UNIQUE,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE repositories (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    owner_id    INTEGER NOT NULL REFERENCES users(id),
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE permissions (
    id          INTEGER PRIMARY KEY,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    repo_id     INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    level       TEXT NOT NULL CHECK(level IN ('read', 'write', 'admin')),
    UNIQUE(user_id, repo_id)
);
```

### 权限模型

- `read`：clone、fetch
- `write`：push（创建分支、更新引用），隐含 read
- `admin`：push + 可通过 CLI 管理该仓库的权限
- owner 自动拥有 admin 权限，不可被 revoke

### 查询流程

用户 SSH 连接 → sshd 匹配公钥后执行 `command="mgs-ssh <fingerprint>"`，原始 git 命令通过 `SSH_ORIGINAL_COMMAND` 环境变量传入 → 查 `ssh_keys` 得到 `user_id` → 根据仓库路径查 `permissions` 判断权限。

owner 权限在查询时计算：如果 `repositories.owner_id == user.id`，则视为 admin，不存储额外的 permissions 行。

## Git 协议处理

```
客户端                              mgs-ssh
  │                                    │
  │  git-upload-pack 'team/repo.git'   │  ← git clone / git fetch
  │───────────────────────────────────▶│
  │                                    │  1. 解析命令和仓库路径
  │                                    │  2. 验证用户权限 (≥ read)
  │                                    │  3. 执行 git-upload-pack
  │  ◀─── pack data ──────────────────│
  │                                    │
  │  git-receive-pack 'team/repo.git'  │  ← git push
  │───────────────────────────────────▶│
  │                                    │  1. 解析命令和仓库路径
  │                                    │  2. 验证用户权限 (≥ write)
  │                                    │  3. 执行 git-receive-pack
  │  ◀─── pack data ──────────────────│
```

直接调用系统 `git-upload-pack` 和 `git-receive-pack`，通过 stdin/stdout 管道连接，不自己实现 pack 协议。

仓库初始化：`mgs repo create` 时执行 `git init --bare`。

## CLI 命令

```
mgs
├── user
│   ├── add <username> --key <pubkey_file>
│   ├── list
│   ├── remove <username>
│   └── key
│       ├── add <username> --key <pubkey_file>
│       ├── list <username>
│       └── remove <fingerprint>
│
├── repo
│   ├── create <name> [--owner <username>]
│   ├── list
│   └── remove <name>
│
├── acl
│   ├── grant <username> <repo> --perm <level>
│   ├── revoke <username> <repo>
│   └── list <repo>
│
└── init                                       # 初始化数据目录、建库、创建 mgs 系统用户（可选）
```

## 错误处理与安全

- 未知公钥 → sshd 直接拒绝连接
- 无权访问仓库 → `Permission denied`
- 仓库不存在 → 返回错误，不泄露其他仓库路径
- 仓库名只允许 `[a-zA-Z0-9/_.-]`，防止路径穿越
- 用户名只允许 `[a-zA-Z0-9_-]`
- 公钥格式校验（类型 + base64 长度）

## 技术选型

| 需求 | Crate |
|------|-------|
| SSH 命令解析 | `shell-words` 或手写 |
| SQLite | `rusqlite` (bundled) |
| CLI 参数 | `clap` |
| 错误处理 | `anyhow` + `thiserror` |
| 进程执行 | `std::process::Command` |
| 文件/路径 | `std::fs`, `std::path` |

不需要 tokio/async、serde、HTTP 库。

## 项目结构

```
mgs/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── db.rs
│   ├── models.rs
│   ├── auth.rs
│   ├── git.rs
│   ├── ssh.rs
│   └── cli/
│       ├── mod.rs
│       ├── user.rs
│       ├── repo.rs
│       ├── acl.rs
│       └── init.rs
├── migrations/
│   └── 001_init.sql
└── tests/
    └── integration.rs
```
