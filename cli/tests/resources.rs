use assert_cmd::Command;
use predicates::str::contains;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// --- Smoke tests: each resource's --help lists its actions and key flags. ---

#[test]
fn wp_help_lists_actions() {
    Command::cargo_bin("laba")
        .unwrap()
        .args(["wp", "--help"])
        .assert()
        .success()
        .stdout(contains("list"))
        .stdout(contains("create"))
        .stdout(contains("delete"));
}

#[test]
fn time_create_help_shows_duration_and_hours() {
    Command::cargo_bin("laba")
        .unwrap()
        .args(["time", "create", "--help"])
        .assert()
        .success()
        .stdout(contains("--duration"))
        .stdout(contains("--hours"));
}

#[test]
fn relation_create_help_shows_type_values() {
    Command::cargo_bin("laba")
        .unwrap()
        .args(["relation", "create", "--help"])
        .assert()
        .success()
        .stdout(contains("relates"))
        .stdout(contains("blocks"));
}

#[test]
fn api_help_shows_fields_and_input() {
    Command::cargo_bin("laba")
        .unwrap()
        .args(["api", "--help"])
        .assert()
        .success()
        .stdout(contains("--field"))
        .stdout(contains("--raw-field"))
        .stdout(contains("--input"));
}

#[test]
fn comment_help_lists_actions() {
    Command::cargo_bin("laba")
        .unwrap()
        .args(["comment", "--help"])
        .assert()
        .success()
        .stdout(contains("create"))
        .stdout(contains("update"));
}

#[test]
fn attachment_help_lists_download() {
    Command::cargo_bin("laba")
        .unwrap()
        .args(["attachment", "--help"])
        .assert()
        .success()
        .stdout(contains("download"))
        .stdout(contains("upload"));
}

#[test]
fn notification_help_lists_actions() {
    Command::cargo_bin("laba")
        .unwrap()
        .args(["notification", "--help"])
        .assert()
        .success()
        .stdout(contains("read"))
        .stdout(contains("unread"));
}

// --- api field parse error: missing '=' exits with code 2. ---

#[test]
fn api_field_without_equals_is_usage_error() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("config.json");
    std::fs::write(
        &cfg,
        json!({
            "default_server": "primary",
            "servers": {"primary": {"base_url": "https://h/openproject"}}
        })
        .to_string(),
    )
    .unwrap();

    Command::cargo_bin("laba")
        .unwrap()
        .env("OPENPROJECT_CACHE", dir.path())
        .args([
            "--config",
            cfg.to_str().unwrap(),
            "--token",
            "t",
            "api",
            "GET",
            "x",
            "-f",
            "bad",
        ])
        .assert()
        .code(2)
        .stderr(contains("Invalid field"));
}

// --- E2E against wiremock. ---

fn write_config(dir: &std::path::Path, base_url: &str) -> std::path::PathBuf {
    let cfg = dir.join("config.json");
    std::fs::write(
        &cfg,
        json!({
            "default_server": "primary",
            "servers": {"primary": {"base_url": base_url, "verify_ssl": false}}
        })
        .to_string(),
    )
    .unwrap();
    cfg
}

#[tokio::test]
async fn wp_list_returns_normalized_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/work_packages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_embedded": {"elements": [{
                "id": 1,
                "subject": "S",
                "updatedAt": "2026-01-01T00:00:00Z",
                "_links": {"status": {"title": "New"}}
            }]}
        })))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path(), &server.uri());

    Command::cargo_bin("laba")
        .unwrap()
        .env("OPENPROJECT_CACHE", dir.path())
        .env("OPENPROJECT_PROXY", "none")
        .args([
            "--config",
            cfg.to_str().unwrap(),
            "--token",
            "t",
            "wp",
            "list",
        ])
        .assert()
        .success()
        .stdout(contains("\"id\": 1"))
        .stdout(contains("customFields"));
}

#[tokio::test]
async fn time_create_sends_iso_duration() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v3/time_entries"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id": 1})))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path(), &server.uri());

    Command::cargo_bin("laba")
        .unwrap()
        .env("OPENPROJECT_CACHE", dir.path())
        .env("OPENPROJECT_PROXY", "none")
        .args([
            "--config",
            cfg.to_str().unwrap(),
            "--token",
            "t",
            "time",
            "create",
            "--work-package",
            "5",
            "--duration",
            "90m",
            "--spent-on",
            "2026-01-01",
        ])
        .assert()
        .success();

    let requests = server.received_requests().await.unwrap();
    let post = requests
        .iter()
        .find(|r| r.method == wiremock::http::Method::POST)
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&post.body).unwrap();
    assert_eq!(body["hours"], json!("PT1H30M"));
}

#[tokio::test]
async fn api_get_sends_query_field() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/projects"))
        .and(query_param("a", "b"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .expect(1)
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path(), &server.uri());

    Command::cargo_bin("laba")
        .unwrap()
        .env("OPENPROJECT_CACHE", dir.path())
        .env("OPENPROJECT_PROXY", "none")
        .args([
            "--config",
            cfg.to_str().unwrap(),
            "--token",
            "t",
            "api",
            "GET",
            "projects",
            "-f",
            "a=b",
        ])
        .assert()
        .success()
        .stdout(contains("\"ok\": true"));
}
