//! HTTP transport for Git Smart HTTP protocol.
//!
//! Provides an HTTP server that handles Git clone/fetch/push operations
//! using token-based authentication.

use anyhow::{Context, Result};
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;

use crate::db::Database;
use crate::git::{repo_disk_path, validate_repo_name};

/// Shared application state for the HTTP server.
struct AppState {
    data_dir: PathBuf,
    db: std::sync::Mutex<Database>,
}

/// Extracts the token from HTTP Basic Auth header.
///
/// Supports two formats:
/// - `http://<token>@host/repo` → username is the token, password empty
/// - `http://user:<token>@host/repo` → password is the token
pub fn extract_token(headers: &HeaderMap) -> Option<String> {
    let auth = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let encoded = auth.strip_prefix("Basic ")?;
    let decoded = STANDARD.decode(encoded).ok()?;
    let decoded = String::from_utf8(decoded).ok()?;
    let (username, password) = decoded.split_once(':')?;
    // Token can be in either username or password field
    if !password.is_empty() {
        Some(password.to_string())
    } else if !username.is_empty() {
        Some(username.to_string())
    } else {
        None
    }
}

/// Authenticates a request and returns the user.
///
/// Checks HTTP Basic Auth where the password is the user's token.
async fn authenticate(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::models::User, Response> {
    let token = extract_token(headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Basic realm=\"mgs\"")],
            "authentication required",
        )
            .into_response()
    })?;

    let db = state.db.lock().unwrap();
    db.find_user_by_token(&token)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "invalid token").into_response())
}

/// Handles GET requests. Dispatches to info_refs based on URL path.
async fn handle_get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(path): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Response, Response> {
    let _user = authenticate(&state, &headers).await?;

    // Parse: "<repo>/info/refs"
    let (repo, action) = parse_path(&path).map_err(|e| e.into_response())?;
    if action != "info/refs" {
        return Err((StatusCode::NOT_FOUND, "not found").into_response());
    }

    let service = params
        .get("service")
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing service parameter").into_response())?;

    let git_cmd = match service.as_str() {
        "git-upload-pack" => "git-upload-pack",
        "git-receive-pack" => "git-receive-pack",
        _ => return Err((StatusCode::BAD_REQUEST, "unsupported service").into_response()),
    };

    let repo_name = repo.strip_suffix(".git").unwrap_or(repo);
    validate_repo_name(repo_name)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()).into_response())?;

    let disk_path = repo_disk_path(&state.data_dir, repo_name);
    if !disk_path.exists() {
        return Err((StatusCode::NOT_FOUND, "repository not found").into_response());
    }

    let output = Command::new(git_cmd)
        .arg("--stateless-rpc")
        .arg("--advertise-refs")
        .arg(&disk_path)
        .output()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, stderr.to_string()).into_response());
    }

    let content_type = format!("application/x-{}-advertisement", service);
    let service_line = format!("# service={}\n", service);
    let service_len = service_line.len() + 4; // 4 hex digits + flush

    let mut body = Vec::new();
    body.extend_from_slice(format!("{:04x}", service_len).as_bytes());
    body.extend_from_slice(service_line.as_bytes());
    body.extend_from_slice(b"0000"); // flush
    body.extend_from_slice(&output.stdout);

    Ok((StatusCode::OK, [(header::CONTENT_TYPE, content_type)], body).into_response())
}

/// Handles POST requests. Dispatches to git-upload-pack or git-receive-pack.
async fn handle_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(path): axum::extract::Path<String>,
    body: Body,
) -> Result<Response, Response> {
    let _user = authenticate(&state, &headers).await?;

    let (repo, action) = parse_path(&path).map_err(|e| e.into_response())?;

    let git_cmd = match action {
        "git-upload-pack" => "git-upload-pack",
        "git-receive-pack" => "git-receive-pack",
        _ => return Err((StatusCode::NOT_FOUND, "not found").into_response()),
    };

    let repo_name = repo.strip_suffix(".git").unwrap_or(repo);
    validate_repo_name(repo_name)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()).into_response())?;

    let disk_path = repo_disk_path(&state.data_dir, repo_name);
    if !disk_path.exists() {
        return Err((StatusCode::NOT_FOUND, "repository not found").into_response());
    }

    git_service_handler(git_cmd, &disk_path, body).await
}

/// Parses a path like "repo/info/refs" into ("repo", "info/refs").
///
/// For nested repos like "team/backend/info/refs", returns ("team/backend", "info/refs").
pub fn parse_path(path: &str) -> Result<(&str, &str), StatusCode> {
    // Find the last known action suffix
    for suffix in &["/info/refs", "/git-upload-pack", "/git-receive-pack"] {
        if let Some(repo_end) = path.strip_suffix(suffix) {
            let repo = repo_end.trim_end_matches('/');
            if !repo.is_empty() {
                return Ok((repo, suffix.trim_start_matches('/')));
            }
        }
    }
    Err(StatusCode::NOT_FOUND)
}

/// Runs a git service command, piping the request body to stdin and capturing stdout.
async fn git_service_handler(
    cmd: &str,
    repo_path: &std::path::Path,
    body: Body,
) -> Result<Response, Response> {
    let mut child = Command::new(cmd)
        .arg("--stateless-rpc")
        .arg(repo_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    // Write request body to stdin
    let stdin = child.stdin.take().unwrap();
    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    let mut stdin = stdin;
    tokio::io::AsyncWriteExt::write_all(&mut stdin, &body_bytes)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    drop(stdin);

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, stderr.to_string()).into_response());
    }

    let content_type = format!("application/x-{}-result", cmd);
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type)],
        output.stdout,
    )
        .into_response())
}

/// Starts the HTTP server on the given address.
pub async fn serve(data_dir: PathBuf, bind: &str) -> Result<()> {
    let db_path = data_dir.join("mgs.db");
    let db = Database::open(&db_path)?;

    let state = Arc::new(AppState {
        data_dir,
        db: std::sync::Mutex::new(db),
    });

    let app = Router::new()
        .route("/{*path}", get(handle_get).post(handle_post))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .with_context(|| format!("failed to bind to {}", bind))?;

    println!("mgs HTTP server listening on {}", bind);

    axum::serve(listener, app).await.context("server error")?;

    Ok(())
}
