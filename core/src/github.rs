//! GitHub backend (read-only) driven through the `gh` CLI.
//!
//! First slice: list my open issues and pull requests, and my notifications.
//! `gh` handles authentication and host selection, so no token is stored here.
//! Output is normalized to the same shape as an OpenProject work package so the
//! CLI and GUI can render both backends uniformly.

use serde::Serialize;
use serde_json::{Map, Value};

use crate::error::Error;

/// Availability of the `gh` CLI, which the GitHub task backend requires. The
/// update checker does NOT use `gh` (it reads public releases anonymously), so
/// this only matters when a GitHub server is configured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GhStatus {
    /// `gh` is installed and authenticated — the backend can run.
    Ready,
    /// `gh` is not on `PATH`; it must be installed.
    Missing,
    /// `gh` is installed but not logged in; `gh auth login` is needed.
    Unauthenticated,
}

/// Classify `gh` availability by probing a [`GhRunner`]: the binary must be
/// present (`gh --version`) and authenticated (`gh auth status`). Separated from
/// process spawning so it is unit-tested with a fake runner.
pub fn gh_status<R: GhRunner>(runner: &R) -> GhStatus {
    match runner.run(&["--version"]) {
        Ok(_) => {}
        // A spawn failure means the binary is absent from PATH.
        Err(Error::Io(_)) => return GhStatus::Missing,
        // Any other failure of `--version` means `gh` is unusable here.
        Err(_) => return GhStatus::Missing,
    }
    match runner.run(&["auth", "status"]) {
        Ok(_) => GhStatus::Ready,
        Err(_) => GhStatus::Unauthenticated,
    }
}

/// Probe the real `gh` for `host` (empty = default host). Convenience over
/// [`gh_status`] with a [`GhCli`] runner.
pub fn gh_status_for_host(host: &str) -> GhStatus {
    gh_status(&GhCli {
        host: host.to_owned(),
    })
}

/// Abstraction over invoking `gh`, so tests can feed fixtures instead of
/// spawning the real process.
pub trait GhRunner {
    /// Run `gh` with `args`, returning captured stdout on success.
    fn run(&self, args: &[&str]) -> Result<Vec<u8>, Error>;
}

/// Real runner: spawns `gh`, pinning the host via `GH_HOST` (github.com or an
/// enterprise host). Command/flag specifics are verified against the installed
/// `gh` at integration time.
pub struct GhCli {
    pub host: String,
}

impl GhRunner for GhCli {
    fn run(&self, args: &[&str]) -> Result<Vec<u8>, Error> {
        let mut cmd = std::process::Command::new("gh");
        cmd.args(args);
        if !self.host.is_empty() {
            cmd.env("GH_HOST", &self.host);
        }
        let out = cmd
            .output()
            .map_err(|e| Error::Io(format!("spawn gh: {e}")))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(Error::Api(format!(
                "gh {}: {}",
                args.join(" "),
                stderr.trim()
            )));
        }
        Ok(out.stdout)
    }
}

/// Whether a searched item is an issue or a pull request; sets the `type` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    Issue,
    PullRequest,
}

impl TaskKind {
    fn type_label(self) -> &'static str {
        match self {
            TaskKind::Issue => "Issue",
            TaskKind::PullRequest => "Pull request",
        }
    }
}

/// Join assignee logins into a single `", "`-separated string, or `Null` when
/// there are none.
fn assignees_label(v: &Value) -> Value {
    let logins: Vec<&str> = v
        .get("assignees")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.get("login").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default();
    if logins.is_empty() {
        Value::Null
    } else {
        Value::from(logins.join(", "))
    }
}

/// Normalize one `gh search issues`/`gh search prs` element to the shared task
/// shape (`id`, `subject`, `type`, `status`, `project`, `assignee`, `dueDate`,
/// `createdAt`, `updatedAt`, `url`).
pub fn normalize_task(v: &Value, kind: TaskKind) -> Value {
    let repo = v
        .get("repository")
        .and_then(|r| r.get("nameWithOwner"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let number = v.get("number").cloned().unwrap_or(Value::Null);
    let id = match number.as_i64() {
        Some(n) if !repo.is_empty() => Value::from(format!("{repo}#{n}")),
        _ => number.clone(),
    };

    let mut m = Map::new();
    m.insert("id".into(), id);
    m.insert(
        "subject".into(),
        v.get("title").cloned().unwrap_or(Value::Null),
    );
    m.insert("type".into(), Value::from(kind.type_label()));
    m.insert(
        "status".into(),
        v.get("state").cloned().unwrap_or(Value::Null),
    );
    m.insert("project".into(), Value::from(repo));
    m.insert("assignee".into(), assignees_label(v));
    m.insert("dueDate".into(), Value::Null);
    m.insert(
        "createdAt".into(),
        v.get("createdAt").cloned().unwrap_or(Value::Null),
    );
    m.insert(
        "updatedAt".into(),
        v.get("updatedAt").cloned().unwrap_or(Value::Null),
    );
    m.insert("url".into(), v.get("url").cloned().unwrap_or(Value::Null));
    Value::Object(m)
}

/// Normalize one element of `gh api notifications` (GitHub REST) to a compact
/// notification shape.
pub fn normalize_notification(v: &Value) -> Value {
    let subject = v.get("subject");
    let mut m = Map::new();
    m.insert("id".into(), v.get("id").cloned().unwrap_or(Value::Null));
    m.insert(
        "reason".into(),
        v.get("reason").cloned().unwrap_or(Value::Null),
    );
    m.insert(
        "subject".into(),
        subject
            .and_then(|s| s.get("title"))
            .cloned()
            .unwrap_or(Value::Null),
    );
    m.insert(
        "type".into(),
        subject
            .and_then(|s| s.get("type"))
            .cloned()
            .unwrap_or(Value::Null),
    );
    m.insert(
        "project".into(),
        v.get("repository")
            .and_then(|r| r.get("full_name"))
            .cloned()
            .unwrap_or(Value::Null),
    );
    m.insert(
        "updatedAt".into(),
        v.get("updated_at").cloned().unwrap_or(Value::Null),
    );
    m.insert(
        "url".into(),
        subject
            .and_then(|s| s.get("url"))
            .cloned()
            .unwrap_or(Value::Null),
    );
    // A browser URL for the notification's subject. `subject.url` is the REST API
    // address (`api.github.com/repos/O/R/issues/N`), which is not viewable in a
    // browser, so derive the web address from the repository and the issue/PR
    // number instead. Only Issue and PullRequest map cleanly; other subject types
    // (Discussion, Release, CheckSuite, …) get no link and stay plain text.
    m.insert(
        "htmlUrl".into(),
        notification_html_url(v)
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    // For CI (CheckSuite) notifications, classify the run outcome from the
    // subject title so the UI can tint it: a failed run reads as a warning, a
    // successful run as good. Absent for every other subject type.
    if subject.and_then(|s| s.get("type")).and_then(Value::as_str) == Some("CheckSuite") {
        let title = subject
            .and_then(|s| s.get("title"))
            .and_then(Value::as_str)
            .unwrap_or("");
        m.insert("outcome".into(), Value::from(check_suite_outcome(title)));
    }
    Value::Object(m)
}

/// Classify a CheckSuite notification's outcome from its subject title, which
/// GitHub phrases as "… workflow run failed/succeeded/cancelled for … branch".
/// Returns `"failure"`, `"success"`, or `"neutral"` so the UI can tint it
/// (failed → warn, succeeded → success). The `gh api notifications` payload
/// carries no structured conclusion, so the title text is the only signal.
fn check_suite_outcome(title: &str) -> &'static str {
    let t = title.to_ascii_lowercase();
    if t.contains("fail") {
        "failure"
    } else if t.contains("succe") || t.contains("passed") {
        "success"
    } else {
        "neutral"
    }
}

/// Browser URL for a notification's subject, or `None` when it cannot be built
/// (unsupported subject type, or missing repository/number). Built from the
/// repository web address so it works on GitHub Enterprise hosts too, not only
/// `github.com`.
fn notification_html_url(v: &Value) -> Option<String> {
    let subject = v.get("subject")?;
    let repo = v.get("repository")?;
    let base = repo
        .get("html_url")
        .and_then(Value::as_str)
        .map(|u| u.trim_end_matches('/').to_string())
        .or_else(|| {
            repo.get("full_name")
                .and_then(Value::as_str)
                .map(|fname| format!("https://github.com/{fname}"))
        })?;
    match subject.get("type").and_then(Value::as_str)? {
        // Issue/PullRequest map to their web page via the subject number.
        ty @ ("Issue" | "PullRequest") => {
            let path = if ty == "Issue" { "issues" } else { "pull" };
            // Number is the last path segment of the subject API URL.
            let number = subject
                .get("url")
                .and_then(Value::as_str)?
                .rsplit('/')
                .next()
                .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))?;
            Some(format!("{base}/{path}/{number}"))
        }
        // CI (CheckSuite) notifications carry no subject number or browser URL,
        // so link to the repository's Actions page where the failed run is listed.
        "CheckSuite" => Some(format!("{base}/actions")),
        // Other subject types (Discussion, Release, …) have no reliable link.
        _ => None,
    }
}

fn parse_tasks(raw: &[u8], kind: TaskKind) -> Result<Vec<Value>, Error> {
    let arr: Vec<Value> =
        serde_json::from_slice(raw).map_err(|e| Error::Api(format!("parse gh output: {e}")))?;
    Ok(arr.iter().map(|v| normalize_task(v, kind)).collect())
}

/// Drop duplicate tasks that surfaced from more than one search, keeping the
/// first occurrence. Keyed by the normalized `id` (`owner/repo#N`), falling back
/// to `url` when an id is absent, so order (issues before PRs) is preserved.
fn dedup_by_id(tasks: Vec<Value>) -> Vec<Value> {
    let mut seen = std::collections::HashSet::new();
    tasks
        .into_iter()
        .filter(|t| {
            let key = t
                .get("id")
                .or_else(|| t.get("url"))
                .map(|v| v.to_string())
                .unwrap_or_default();
            seen.insert(key)
        })
        .collect()
}

/// Read-only GitHub backend over a [`GhRunner`].
pub struct GithubBackend<R: GhRunner> {
    runner: R,
}

impl<R: GhRunner> GithubBackend<R> {
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    /// The authenticated user's login, for owner-scoped searches (`gh search`
    /// takes a concrete login for `--owner`, not `@me`).
    fn my_login(&self) -> Result<String, Error> {
        let raw = self.runner.run(&["api", "user", "--jq", ".login"])?;
        Ok(String::from_utf8_lossy(&raw).trim().to_string())
    }

    /// Everything on GitHub that needs my attention, aggregated and de-duplicated:
    /// issues/PRs I'm involved in (author, assignee, mention, comment), PRs whose
    /// review is requested from me, and everything open in my own repositories.
    /// The same item surfacing in several searches is collapsed by `id`.
    pub fn list_my_tasks(&self) -> Result<Vec<Value>, Error> {
        const ISSUE_FIELDS: &str =
            "number,title,state,repository,assignees,createdAt,updatedAt,url";
        const PR_FIELDS: &str =
            "number,title,state,repository,assignees,createdAt,updatedAt,url,isDraft";
        let login = self.my_login()?;

        let mut out = Vec::new();
        // Issues: anything I'm involved in, plus everything open in my repos.
        for args in [
            vec![
                "search",
                "issues",
                "--involves",
                "@me",
                "--state",
                "open",
                "--json",
                ISSUE_FIELDS,
            ],
            vec![
                "search",
                "issues",
                "--owner",
                &login,
                "--state",
                "open",
                "--json",
                ISSUE_FIELDS,
            ],
        ] {
            out.extend(parse_tasks(&self.runner.run(&args)?, TaskKind::Issue)?);
        }
        // PRs: involving me, review requested from me, plus everything in my repos.
        for args in [
            vec![
                "search",
                "prs",
                "--involves",
                "@me",
                "--state",
                "open",
                "--json",
                PR_FIELDS,
            ],
            vec![
                "search",
                "prs",
                "--review-requested",
                "@me",
                "--state",
                "open",
                "--json",
                PR_FIELDS,
            ],
            vec![
                "search", "prs", "--owner", &login, "--state", "open", "--json", PR_FIELDS,
            ],
        ] {
            out.extend(parse_tasks(
                &self.runner.run(&args)?,
                TaskKind::PullRequest,
            )?);
        }
        Ok(dedup_by_id(out))
    }

    /// My GitHub notifications, normalized.
    pub fn list_notifications(&self) -> Result<Vec<Value>, Error> {
        let raw = self.runner.run(&["api", "notifications"])?;
        let arr: Vec<Value> = serde_json::from_slice(&raw)
            .map_err(|e| Error::Api(format!("parse gh output: {e}")))?;
        Ok(arr.iter().map(normalize_notification).collect())
    }
}

/// Test-only fake `gh` runner, shared with the `backend` facade tests.
#[cfg(test)]
pub mod tests_support {
    use super::{Error, GhRunner};

    /// Fake runner returning canned fixtures keyed by the leading args.
    pub struct FakeGh {
        issues: Vec<u8>,
        prs: Vec<u8>,
        notifications: Vec<u8>,
    }

    impl FakeGh {
        pub fn new(issues: Vec<u8>, prs: Vec<u8>, notifications: Vec<u8>) -> Self {
            Self {
                issues,
                prs,
                notifications,
            }
        }
    }

    impl GhRunner for FakeGh {
        fn run(&self, args: &[&str]) -> Result<Vec<u8>, Error> {
            match (args.first().copied(), args.get(1).copied()) {
                (Some("search"), Some("issues")) => Ok(self.issues.clone()),
                (Some("search"), Some("prs")) => Ok(self.prs.clone()),
                // `gh api user` (login lookup) vs `gh api notifications`.
                (Some("api"), Some("user")) => Ok(b"testuser".to_vec()),
                (Some("api"), Some("notifications")) => Ok(self.notifications.clone()),
                _ => Err(Error::Api(format!("unexpected gh args: {args:?}"))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::tests_support::FakeGh;
    use super::*;
    use serde_json::json;

    /// Fake runner whose `--version` / `auth status` outcomes are configurable,
    /// to classify [`gh_status`] without the real binary.
    struct StatusGh {
        installed: bool,
        authed: bool,
    }

    impl GhRunner for StatusGh {
        fn run(&self, args: &[&str]) -> Result<Vec<u8>, Error> {
            match args.first().copied() {
                Some("--version") if self.installed => Ok(Vec::new()),
                Some("--version") => Err(Error::Io("spawn gh: not found".into())),
                Some("auth") if self.authed => Ok(Vec::new()),
                Some("auth") => Err(Error::Api("not logged in".into())),
                _ => Err(Error::Api(format!("unexpected gh args: {args:?}"))),
            }
        }
    }

    #[test]
    fn gh_status_classifies_missing_unauth_ready() {
        assert_eq!(
            gh_status(&StatusGh {
                installed: false,
                authed: false
            }),
            GhStatus::Missing
        );
        assert_eq!(
            gh_status(&StatusGh {
                installed: true,
                authed: false
            }),
            GhStatus::Unauthenticated
        );
        assert_eq!(
            gh_status(&StatusGh {
                installed: true,
                authed: true
            }),
            GhStatus::Ready
        );
    }

    #[test]
    fn normalize_issue_maps_shared_shape() {
        let v = json!({
            "number": 42,
            "title": "Fix the widget",
            "state": "open",
            "repository": {"name": "app", "nameWithOwner": "acme/app"},
            "assignees": [{"login": "me"}, {"login": "you"}],
            "createdAt": "2026-07-01T00:00:00Z",
            "updatedAt": "2026-07-02T00:00:00Z",
            "url": "https://github.com/acme/app/issues/42"
        });
        let n = normalize_task(&v, TaskKind::Issue);
        assert_eq!(n["id"], json!("acme/app#42"));
        assert_eq!(n["subject"], json!("Fix the widget"));
        assert_eq!(n["type"], json!("Issue"));
        assert_eq!(n["status"], json!("open"));
        assert_eq!(n["project"], json!("acme/app"));
        assert_eq!(n["assignee"], json!("me, you"));
        assert_eq!(n["dueDate"], Value::Null);
        assert_eq!(n["url"], json!("https://github.com/acme/app/issues/42"));
    }

    #[test]
    fn normalize_pr_sets_type_and_handles_no_assignee() {
        let v = json!({
            "number": 7,
            "title": "Add caching",
            "state": "open",
            "repository": {"nameWithOwner": "acme/app"},
            "assignees": [],
            "url": "https://github.com/acme/app/pull/7"
        });
        let n = normalize_task(&v, TaskKind::PullRequest);
        assert_eq!(n["id"], json!("acme/app#7"));
        assert_eq!(n["type"], json!("Pull request"));
        assert_eq!(n["assignee"], Value::Null);
    }

    #[test]
    fn normalize_notification_maps_fields() {
        let v = json!({
            "id": "9001",
            "reason": "mention",
            "subject": {"title": "Ping", "type": "Issue", "url": "https://api.github.com/repos/acme/app/issues/42"},
            "repository": {"full_name": "acme/app", "html_url": "https://github.com/acme/app"},
            "updated_at": "2026-07-02T00:00:00Z"
        });
        let n = normalize_notification(&v);
        assert_eq!(n["id"], json!("9001"));
        assert_eq!(n["reason"], json!("mention"));
        assert_eq!(n["subject"], json!("Ping"));
        assert_eq!(n["type"], json!("Issue"));
        assert_eq!(n["project"], json!("acme/app"));
        assert_eq!(n["htmlUrl"], json!("https://github.com/acme/app/issues/42"));
    }

    #[test]
    fn normalize_notification_html_url_for_pr_and_fallback() {
        // PullRequest maps to the `/pull/N` web path (not the API `/pulls/N`).
        let pr = json!({
            "subject": {"title": "PR", "type": "PullRequest", "url": "https://api.github.com/repos/acme/app/pulls/7"},
            "repository": {"full_name": "acme/app", "html_url": "https://github.com/acme/app"}
        });
        assert_eq!(
            normalize_notification(&pr)["htmlUrl"],
            json!("https://github.com/acme/app/pull/7")
        );
        // No html_url on the repository: fall back to github.com/<full_name>.
        let fallback = json!({
            "subject": {"title": "I", "type": "Issue", "url": "https://api.github.com/repos/acme/app/issues/3"},
            "repository": {"full_name": "acme/app"}
        });
        assert_eq!(
            normalize_notification(&fallback)["htmlUrl"],
            json!("https://github.com/acme/app/issues/3")
        );
        // Unsupported subject type: no link.
        let disc = json!({
            "subject": {"title": "D", "type": "Discussion", "url": "https://api.github.com/repos/acme/app/discussions/1"},
            "repository": {"full_name": "acme/app"}
        });
        assert_eq!(normalize_notification(&disc)["htmlUrl"], json!(null));
    }

    #[test]
    fn normalize_notification_check_suite_links_to_actions() {
        // CI (CheckSuite) notifications carry no subject number/url; they link to
        // the repository's Actions page. Uses the repository html_url as the base.
        let ci = json!({
            "subject": {"title": "CI workflow run failed", "type": "CheckSuite", "url": null},
            "repository": {"full_name": "acme/app", "html_url": "https://github.com/acme/app"}
        });
        assert_eq!(
            normalize_notification(&ci)["htmlUrl"],
            json!("https://github.com/acme/app/actions")
        );
        // Enterprise host / no repo html_url: fall back to github.com/<full_name>.
        let ci_fallback = json!({
            "subject": {"title": "CI", "type": "CheckSuite"},
            "repository": {"full_name": "acme/app"}
        });
        assert_eq!(
            normalize_notification(&ci_fallback)["htmlUrl"],
            json!("https://github.com/acme/app/actions")
        );
    }

    #[test]
    fn check_suite_outcome_classifies_by_title() {
        assert_eq!(
            check_suite_outcome("CI workflow run failed for main branch"),
            "failure"
        );
        assert_eq!(
            check_suite_outcome("CI workflow run succeeded for main branch"),
            "success"
        );
        assert_eq!(check_suite_outcome("All checks passed"), "success");
        assert_eq!(check_suite_outcome("CI workflow run cancelled"), "neutral");
        assert_eq!(check_suite_outcome(""), "neutral");
    }

    #[test]
    fn normalize_notification_sets_ci_outcome_only_for_check_suite() {
        // Failed CI run -> outcome "failure".
        let fail = json!({
            "subject": {"title": "CI workflow run failed for x", "type": "CheckSuite"},
            "repository": {"full_name": "acme/app"}
        });
        assert_eq!(normalize_notification(&fail)["outcome"], json!("failure"));
        // Successful CI run -> outcome "success".
        let ok = json!({
            "subject": {"title": "CI workflow run succeeded for x", "type": "CheckSuite"},
            "repository": {"full_name": "acme/app"}
        });
        assert_eq!(normalize_notification(&ok)["outcome"], json!("success"));
        // Non-CI notifications carry no outcome field.
        let issue = json!({
            "subject": {"title": "Ping", "type": "Issue", "url": "https://api.github.com/repos/acme/app/issues/1"},
            "repository": {"full_name": "acme/app"}
        });
        assert_eq!(normalize_notification(&issue).get("outcome"), None);
    }

    #[test]
    fn list_my_tasks_merges_issues_and_prs() {
        let fake = FakeGh::new(
            json!([{
                "number": 1, "title": "I1", "state": "open",
                "repository": {"nameWithOwner": "acme/app"}, "assignees": [{"login": "me"}]
            }])
            .to_string()
            .into_bytes(),
            json!([{
                "number": 2, "title": "P2", "state": "open",
                "repository": {"nameWithOwner": "acme/app"}, "assignees": []
            }])
            .to_string()
            .into_bytes(),
            b"[]".to_vec(),
        );
        // The same issue/PR fixture is returned by several searches (involves,
        // owner, review-requested); dedup_by_id collapses each to one entry.
        let out = GithubBackend::new(fake).list_my_tasks().unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["type"], json!("Issue"));
        assert_eq!(out[0]["id"], json!("acme/app#1"));
        assert_eq!(out[1]["type"], json!("Pull request"));
        assert_eq!(out[1]["id"], json!("acme/app#2"));
    }

    #[test]
    fn dedup_by_id_keeps_first_occurrence_and_order() {
        let a1 = json!({"id": "acme/app#1", "subject": "first"});
        let a1_dup = json!({"id": "acme/app#1", "subject": "again"});
        let b2 = json!({"id": "acme/app#2", "subject": "second"});
        let out = dedup_by_id(vec![a1.clone(), b2.clone(), a1_dup]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["id"], json!("acme/app#1"));
        assert_eq!(out[0]["subject"], json!("first"));
        assert_eq!(out[1]["id"], json!("acme/app#2"));
    }

    #[test]
    fn list_notifications_normalizes() {
        let fake = FakeGh::new(
            b"[]".to_vec(),
            b"[]".to_vec(),
            json!([{
                "id": "1", "reason": "assign",
                "subject": {"title": "T", "type": "PullRequest", "url": "u"},
                "repository": {"full_name": "acme/app"}, "updated_at": "2026-07-02T00:00:00Z"
            }])
            .to_string()
            .into_bytes(),
        );
        let out = GithubBackend::new(fake).list_notifications().unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["subject"], json!("T"));
        assert_eq!(out[0]["project"], json!("acme/app"));
    }
}
