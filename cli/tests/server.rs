use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn add_list_setdefault_remove() {
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
        .success()
        .stdout(contains("added server 'primary'"));

    Command::cargo_bin("laba")
        .unwrap()
        .args(["--config", cfg.to_str().unwrap(), "server", "list"])
        .assert()
        .success()
        .stdout(contains("\"default\": true"));

    Command::cargo_bin("laba")
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
fn proxy_set_show_and_clear_for_server_and_global() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("config.json");
    let run = |args: &[&str]| {
        let mut full = vec!["--config", cfg.to_str().unwrap()];
        full.extend_from_slice(args);
        Command::cargo_bin("laba").unwrap().args(full).assert()
    };

    run(&["server", "add", "primary", "--url", "https://h/openproject"]).success();

    // Per-server override: set, show, then clear (defaults to the default server).
    run(&["server", "proxy", "socks5://p:1"])
        .success()
        .stdout(contains("proxy for 'primary' -> socks5://p:1"));
    run(&["server", "proxy"])
        .success()
        .stdout(contains("socks5://p:1"));
    // An empty value clears the override (inherit global/env).
    run(&["server", "proxy", ""])
        .success()
        .stdout(contains("cleared proxy override for 'primary'"));
    run(&["server", "proxy"]).success().stdout("\n");

    // Global default: set, show, then clear via --clear.
    run(&["server", "global-proxy", "http://g:2"])
        .success()
        .stdout(contains("global proxy -> http://g:2"));
    run(&["server", "global-proxy"])
        .success()
        .stdout(contains("http://g:2"));
    run(&["server", "global-proxy", "--clear"])
        .success()
        .stdout(contains("cleared global proxy"));
    run(&["server", "global-proxy"]).success().stdout("\n");
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
        Command::cargo_bin("laba").unwrap().args(args).assert()
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

    Command::cargo_bin("laba")
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
