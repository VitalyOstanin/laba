use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn cache_clear_all_reports_cleared() {
    let cache_dir = tempfile::tempdir().unwrap();

    Command::cargo_bin("laboro")
        .unwrap()
        .env("OPENPROJECT_CACHE", cache_dir.path())
        .args(["cache", "clear", "--all"])
        .assert()
        .success()
        .stdout(contains("\"cleared\": \"all\""));
}

#[test]
fn help_lists_cache_subcommand() {
    Command::cargo_bin("laboro")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("cache"));
}
