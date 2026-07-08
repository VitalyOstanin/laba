use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn add_list_setdefault_remove() {
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
        .success()
        .stdout(contains("added server 'primary'"));

    Command::cargo_bin("taskstream")
        .unwrap()
        .args(["--config", cfg.to_str().unwrap(), "server", "list"])
        .assert()
        .success()
        .stdout(contains("\"default\": true"));

    Command::cargo_bin("taskstream")
        .unwrap()
        .args([
            "--config",
            cfg.to_str().unwrap(),
            "server",
            "remove",
            "primary",
        ])
        .assert()
        .success()
        .stdout(contains("removed server 'primary'"));
}
