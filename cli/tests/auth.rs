use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn login_then_offline_status() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("config.json");

    Command::cargo_bin("taskstream")
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

    Command::cargo_bin("taskstream")
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
