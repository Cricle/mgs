use axum::http::{HeaderMap, StatusCode, header};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use mgs::http::{extract_token, parse_path};

// --- parse_path tests ---

#[test]
fn test_parse_path_info_refs() {
    let (repo, action) = parse_path("testrepo/info/refs").unwrap();
    assert_eq!(repo, "testrepo");
    assert_eq!(action, "info/refs");
}

#[test]
fn test_parse_path_info_refs_with_git() {
    let (repo, action) = parse_path("testrepo.git/info/refs").unwrap();
    assert_eq!(repo, "testrepo.git");
    assert_eq!(action, "info/refs");
}

#[test]
fn test_parse_path_nested_repo() {
    let (repo, action) = parse_path("team/backend/info/refs").unwrap();
    assert_eq!(repo, "team/backend");
    assert_eq!(action, "info/refs");
}

#[test]
fn test_parse_path_nested_repo_with_git() {
    let (repo, action) = parse_path("team/backend.git/info/refs").unwrap();
    assert_eq!(repo, "team/backend.git");
    assert_eq!(action, "info/refs");
}

#[test]
fn test_parse_path_upload_pack() {
    let (repo, action) = parse_path("testrepo/git-upload-pack").unwrap();
    assert_eq!(repo, "testrepo");
    assert_eq!(action, "git-upload-pack");
}

#[test]
fn test_parse_path_receive_pack() {
    let (repo, action) = parse_path("testrepo/git-receive-pack").unwrap();
    assert_eq!(repo, "testrepo");
    assert_eq!(action, "git-receive-pack");
}

#[test]
fn test_parse_path_deeply_nested() {
    let (repo, action) = parse_path("a/b/c/d/info/refs").unwrap();
    assert_eq!(repo, "a/b/c/d");
    assert_eq!(action, "info/refs");
}

#[test]
fn test_parse_path_deeply_nested_upload_pack() {
    let (repo, action) = parse_path("a/b/c/git-upload-pack").unwrap();
    assert_eq!(repo, "a/b/c");
    assert_eq!(action, "git-upload-pack");
}

#[test]
fn test_parse_path_unknown_action() {
    assert!(parse_path("testrepo/unknown").is_err());
}

#[test]
fn test_parse_path_empty() {
    assert!(parse_path("").is_err());
}

#[test]
fn test_parse_path_only_action() {
    assert!(parse_path("info/refs").is_err());
}

#[test]
fn test_parse_path_only_slash() {
    assert!(parse_path("/").is_err());
}

#[test]
fn test_parse_path_action_with_extra_segments() {
    // "testrepo/info/refs/extra" should not match
    assert!(parse_path("testrepo/info/refs/extra").is_err());
}

#[test]
fn test_parse_path_repo_name_with_dots() {
    let (repo, action) = parse_path("my.repo.name/info/refs").unwrap();
    assert_eq!(repo, "my.repo.name");
    assert_eq!(action, "info/refs");
}

#[test]
fn test_parse_path_repo_name_with_dashes() {
    let (repo, action) = parse_path("my-repo/info/refs").unwrap();
    assert_eq!(repo, "my-repo");
    assert_eq!(action, "info/refs");
}

#[test]
fn test_parse_path_single_segment_repo() {
    let (repo, action) = parse_path("repo/git-receive-pack").unwrap();
    assert_eq!(repo, "repo");
    assert_eq!(action, "git-receive-pack");
}

// --- extract_token tests ---

fn make_basic_auth(user: &str, pass: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let encoded = STANDARD.encode(format!("{}:{}", user, pass));
    headers.insert(
        header::AUTHORIZATION,
        axum::http::HeaderValue::from_str(&format!("Basic {}", encoded)).unwrap(),
    );
    headers
}

#[test]
fn test_extract_token_from_username_only() {
    let headers = make_basic_auth("mytoken", "");
    let token = extract_token(&headers).unwrap();
    assert_eq!(token, "mytoken");
}

#[test]
fn test_extract_token_from_password_only() {
    let headers = make_basic_auth("", "mypassword");
    let token = extract_token(&headers).unwrap();
    assert_eq!(token, "mypassword");
}

#[test]
fn test_extract_token_prefers_password() {
    let headers = make_basic_auth("user", "pass");
    let token = extract_token(&headers).unwrap();
    assert_eq!(token, "pass");
}

#[test]
fn test_extract_token_no_auth() {
    let headers = HeaderMap::new();
    assert!(extract_token(&headers).is_none());
}

#[test]
fn test_extract_token_invalid_base64() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        axum::http::HeaderValue::from_static("Basic not-valid-base64!!!"),
    );
    assert!(extract_token(&headers).is_none());
}

#[test]
fn test_extract_token_empty_credentials() {
    let headers = make_basic_auth("", "");
    assert!(extract_token(&headers).is_none());
}

#[test]
fn test_extract_token_not_basic() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        axum::http::HeaderValue::from_static("Bearer sometoken"),
    );
    assert!(extract_token(&headers).is_none());
}

#[test]
fn test_extract_token_long_token() {
    let long_token = "a".repeat(64);
    let headers = make_basic_auth(&long_token, "");
    let token = extract_token(&headers).unwrap();
    assert_eq!(token, long_token);
}

#[test]
fn test_extract_token_hex_token() {
    let hex_token = "bf93fd055f4020dda33559fa6deda9832d8781efcb09da0354b0176bc9b3964b";
    let headers = make_basic_auth(hex_token, "");
    let token = extract_token(&headers).unwrap();
    assert_eq!(token, hex_token);
}

#[test]
fn test_extract_token_with_special_chars_in_password() {
    let headers = make_basic_auth("", "token-with-special!@#$%");
    let token = extract_token(&headers).unwrap();
    assert_eq!(token, "token-with-special!@#$%");
}

#[test]
fn test_extract_token_malformed_header() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        axum::http::HeaderValue::from_static("Basic"),
    );
    assert!(extract_token(&headers).is_none());
}

#[test]
fn test_extract_token_colon_in_password() {
    let headers = make_basic_auth("", "pass:with:colons");
    let token = extract_token(&headers).unwrap();
    assert_eq!(token, "pass:with:colons");
}

#[test]
fn test_extract_token_colon_in_username() {
    // When username contains colon, split_once(':') splits on first colon
    // "user:name:" -> ("user", "name:") -> password is "name:" (non-empty)
    let headers = make_basic_auth("user:name", "");
    let token = extract_token(&headers).unwrap();
    assert_eq!(token, "name:");
}

#[test]
fn test_parse_path_returns_correct_error_type() {
    let result = parse_path("unknown/path");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), StatusCode::NOT_FOUND);
}

#[test]
fn test_parse_path_info_refs_variations() {
    // Test that only /info/refs matches, not other /info/ paths
    assert!(parse_path("repo/info/other").is_err());
    assert!(parse_path("repo/info").is_err());
}

#[test]
fn test_parse_path_upload_pack_not_receive_pack() {
    let (repo, action) = parse_path("repo/git-upload-pack").unwrap();
    assert_eq!(repo, "repo");
    assert_eq!(action, "git-upload-pack");
    assert_ne!(action, "git-receive-pack");
}

#[test]
fn test_parse_path_receive_pack_not_upload_pack() {
    let (repo, action) = parse_path("repo/git-receive-pack").unwrap();
    assert_eq!(repo, "repo");
    assert_eq!(action, "git-receive-pack");
    assert_ne!(action, "git-upload-pack");
}
