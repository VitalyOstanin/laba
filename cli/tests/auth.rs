use assert_cmd::Command;
use predicates::str::contains;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn auth_login_rejects_duplicate_user_unless_forced() {
    let server = MockServer::start().await;
    // Any token validates as the same account.
    Mock::given(method("GET"))
        .and(path("/api/v3/users/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"login": "alice", "id": 1})))
        .mount(&server)
        .await;
    let uri = server.uri();

    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("config.json");
    let secrets = dir.path().join("secrets.json");
    let cfg_s = cfg.to_str().unwrap().to_owned();

    // OPENPROJECT_SECRETS makes the token store a self-contained file (the system
    // keyring is skipped), so this test is isolated locally and in CI alike.
    let run = |args: &[&str]| {
        let mut full = vec!["--config", cfg_s.as_str()];
        full.extend_from_slice(args);
        let mut c = Command::cargo_bin("laba").unwrap();
        c.env("OPENPROJECT_CACHE", dir.path())
            .env("OPENPROJECT_SECRETS", &secrets)
            .env("OPENPROJECT_PROXY", "none")
            .args(full);
        c.assert()
    };

    // Two profiles on the same base URL.
    run(&["server", "add", "primary", "--url", &uri]).success();
    run(&["server", "add", "secondary", "--url", &uri]).success();

    // First login stores the token for primary.
    run(&["--server", "primary", "auth", "login", "--token", "t"]).success();

    // A second login as the same user on the same base URL is rejected.
    run(&["--server", "secondary", "auth", "login", "--token", "t"])
        .failure()
        .stderr(contains(
            "already authenticated as server 'primary'; use --force to add anyway",
        ));

    // --force adds it anyway.
    run(&[
        "--server",
        "secondary",
        "auth",
        "login",
        "--token",
        "t",
        "--force",
    ])
    .success();
}

#[test]
fn login_then_offline_status() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("config.json");

    Command::cargo_bin("laba")
        .unwrap()
        .args([
            "--config",
            cfg.to_str().unwrap(),
            "server",
            "add",
            "primary",
            "--url",
            "https://h/openproject",
        ])
        .assert()
        .success();

    Command::cargo_bin("laba")
        .unwrap()
        .args([
            "--config",
            cfg.to_str().unwrap(),
            "--server",
            "primary",
            "auth",
            "status",
            "--offline",
            "--token",
            "t",
        ])
        .assert()
        .success()
        .stdout(contains("\"hasToken\":true"));
}
