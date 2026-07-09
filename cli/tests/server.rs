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

#[test]
fn add_rejects_duplicate_name_unless_forced() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("config.json");
    let add = |url: &str, extra: &[&str]| {
        let mut args = vec![
            "--config",
            cfg.to_str().unwrap(),
            "server",
            "add",
            "primary",
            "--url",
            url,
        ];
        args.extend_from_slice(extra);
        Command::cargo_bin("taskstream")
            .unwrap()
            .args(args)
            .assert()
    };

    add("https://h/openproject", &[])
        .success()
        .stdout(contains("added server 'primary'"));

    // A second add with the same name is rejected and does not overwrite.
    add("https://other/openproject", &[])
        .failure()
        .stderr(contains("already exists"));

    // --force replaces it.
    add("https://other/openproject", &["--force"])
        .success()
        .stdout(contains("replaced server 'primary'"));

    Command::cargo_bin("taskstream")
        .unwrap()
        .args([
            "--config",
            cfg.to_str().unwrap(),
            "server",
            "show",
            "primary",
        ])
        .assert()
        .success()
        .stdout(contains("https://other/openproject"));
}
