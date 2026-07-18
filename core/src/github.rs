//! GitHub backend driven through the `gh` CLI.
//!
//! Lists my open issues and pull requests and my notifications (read and unread,
//! via `all=true`, each tagged with a `read` flag), and marks notifications read
//! (`PATCH`/`PUT`); GitHub's REST API has no mark-unread, so read is one-way.
//! `gh` handles authentication and host selection, so no token is stored here.
//! Output is normalized to the same shape as an OpenProject work package so the
//! CLI and GUI can render both backends uniformly.

use serde::Serialize;
use serde_json::Value;

use crate::entities;
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

/// Whether a searched item is an issue or a pull request; maps to the typed
/// [`entities::TaskKind`] on the produced task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    Issue,
    PullRequest,
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

/// Assignees as a comma-joined string, or `None` when there are none.
fn assignees_string(v: &Value) -> Option<String> {
    match assignees_label(v) {
        Value::String(s) => Some(s),
        _ => None,
    }
}

/// Map a GitHub issue/PR `state` (`open` / `closed` / `merged`) to a normalized
/// status bucket.
fn status_category_from_state(state: Option<&str>) -> entities::StatusCategory {
    match state {
        Some(s) if s.eq_ignore_ascii_case("open") => entities::StatusCategory::Open,
        Some(s) if s.eq_ignore_ascii_case("closed") || s.eq_ignore_ascii_case("merged") => {
            entities::StatusCategory::Done
        }
        _ => entities::StatusCategory::Unknown,
    }
}

/// Build a typed [`entities::Task`] from one `gh search issues`/`prs` element.
/// `reason` is supplied by the caller, which knows the search that produced the
/// item (involves / review-requested / owner); GitHub search does not tag it.
pub fn task_from_gh(v: &Value, kind: TaskKind, reason: entities::InboxReason) -> entities::Task {
    let repo = v
        .get("repository")
        .and_then(|r| r.get("nameWithOwner"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let number = v.get("number").and_then(Value::as_i64);
    let display = match number {
        Some(n) if !repo.is_empty() => format!("{repo}#{n}"),
        Some(n) => n.to_string(),
        None => String::new(),
    };
    let raw = number.map(|n| n.to_string()).unwrap_or_default();
    let state = v.get("state").and_then(Value::as_str);
    entities::Task {
        id: entities::TaskId { display, raw },
        kind: match kind {
            TaskKind::Issue => entities::TaskKind::Issue,
            TaskKind::PullRequest => entities::TaskKind::PullRequest,
        },
        reason,
        title: v
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned(),
        url: v.get("url").and_then(Value::as_str).map(str::to_owned),
        status: state.map(str::to_owned),
        status_category: status_category_from_state(state),
        project: (!repo.is_empty()).then(|| repo.to_owned()),
        assignee: assignees_string(v),
        author: None,
        created_at: v
            .get("createdAt")
            .and_then(Value::as_str)
            .map(str::to_owned),
        updated_at: v
            .get("updatedAt")
            .and_then(Value::as_str)
            .map(str::to_owned),
        due_date: None,
        priority: None,
        labels: Vec::new(),
        custom_fields: Vec::new(),
    }
}

/// Map a GitHub notification subject type (PascalCase: `Issue`, `PullRequest`,
/// `CheckSuite`, `Discussion`, …) to a [`entities::NotifKind`].
fn notif_kind_from_subject_type(t: Option<&str>) -> entities::NotifKind {
    match t {
        Some("Issue") => entities::NotifKind::Issue,
        Some("PullRequest") => entities::NotifKind::PullRequest,
        Some("CheckSuite") => entities::NotifKind::CheckSuite,
        Some(other) => entities::NotifKind::Other(other.to_owned()),
        None => entities::NotifKind::Other(String::new()),
    }
}

/// The typed CI outcome for a check-suite title (mirrors [`check_suite_outcome`]).
fn ci_outcome_from_title(title: &str) -> entities::CiOutcome {
    match check_suite_outcome(title) {
        "failure" => entities::CiOutcome::Failure,
        "success" => entities::CiOutcome::Success,
        _ => entities::CiOutcome::Neutral,
    }
}

/// Build a typed [`entities::Notification`] from one `gh api notifications`
/// element. `url` is the browser-viewable subject address (the REST `subject.url`
/// is not viewable); a check-suite item also carries its run `outcome`.
pub fn notification_from_gh(v: &Value) -> entities::Notification {
    let subject = v.get("subject");
    let subject_type = subject.and_then(|s| s.get("type")).and_then(Value::as_str);
    let title = subject
        .and_then(|s| s.get("title"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let unread = v.get("unread").and_then(Value::as_bool).unwrap_or(true);
    let outcome = (subject_type == Some("CheckSuite")).then(|| ci_outcome_from_title(&title));
    entities::Notification {
        id: v.get("id").and_then(Value::as_str).unwrap_or("").to_owned(),
        reason: v
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned(),
        kind: notif_kind_from_subject_type(subject_type),
        title,
        project: v
            .get("repository")
            .and_then(|r| r.get("full_name"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        url: notification_html_url(v),
        updated_at: v
            .get("updated_at")
            .and_then(Value::as_str)
            .map(str::to_owned),
        read: !unread,
        outcome,
        wp_id: None,
    }
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

/// Workflow name from a CheckSuite title, phrased "<workflow> workflow run
/// <status> for <branch> branch". Returns the leading workflow name, or `None`
/// when the title lacks the "workflow run" marker.
fn check_suite_workflow(title: &str) -> Option<&str> {
    let idx = title.find(" workflow run")?;
    let wf = title[..idx].trim();
    (!wf.is_empty()).then_some(wf)
}

/// Branch from a CheckSuite title (the token between "for" and the trailing
/// "branch"). Returns the branch (`main`, `feature/x`, …), or `None`.
fn check_suite_branch(title: &str) -> Option<String> {
    let after = title.split(" for ").nth(1)?.trim();
    let branch = after.strip_suffix(" branch").unwrap_or(after).trim();
    (!branch.is_empty()).then(|| branch.to_string())
}

/// Browser URL of the workflow run a CheckSuite notification refers to. Matches
/// `runs` (a repository's `workflow_runs`) by the workflow name and branch parsed
/// from `title`, then picks the run whose `updated_at` is closest to the
/// notification's `notif_updated`. Returns `None` when nothing matches.
fn match_run_url(runs: &[Value], title: &str, notif_updated: &str) -> Option<String> {
    let workflow = check_suite_workflow(title)?;
    let branch = check_suite_branch(title);
    let target = parse_ts(notif_updated);
    runs.iter()
        .filter(|r| r.get("name").and_then(Value::as_str) == Some(workflow))
        .filter(|r| match &branch {
            Some(b) => r.get("head_branch").and_then(Value::as_str) == Some(b.as_str()),
            None => true,
        })
        .min_by_key(|r| {
            let run_ts = r
                .get("updated_at")
                .and_then(Value::as_str)
                .and_then(parse_ts);
            match (target, run_ts) {
                (Some(t), Some(u)) => (t - u).num_seconds().abs(),
                _ => i64::MAX,
            }
        })
        .and_then(|r| {
            r.get("html_url")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

/// Parse an RFC 3339 timestamp, or `None` when it does not parse.
fn parse_ts(s: &str) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    chrono::DateTime::parse_from_rfc3339(s).ok()
}

fn parse_tasks(
    raw: &[u8],
    kind: TaskKind,
    reason: entities::InboxReason,
) -> Result<Vec<entities::Task>, Error> {
    let arr: Vec<Value> =
        serde_json::from_slice(raw).map_err(|e| Error::Api(format!("parse gh output: {e}")))?;
    Ok(arr.iter().map(|v| task_from_gh(v, kind, reason)).collect())
}

/// Drop duplicate tasks that surfaced from more than one search, keeping the
/// first occurrence. Keyed by the display id (`owner/repo#N`), falling back to
/// `url` when it is empty. Ordering matters: the searches are run most-specific
/// reason first (review-requested before involves), so the kept copy carries the
/// most useful "why it's in my list" reason.
fn dedup_by_id(tasks: Vec<entities::Task>) -> Vec<entities::Task> {
    let mut seen = std::collections::HashSet::new();
    tasks
        .into_iter()
        .filter(|t| {
            let key = if t.id.display.is_empty() {
                t.url.clone().unwrap_or_default()
            } else {
                t.id.display.clone()
            };
            seen.insert(key)
        })
        .collect()
}

/// GitHub backend over a [`GhRunner`]: reads tasks/notifications and marks
/// notifications read.
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
    pub fn list_my_tasks(&self) -> Result<Vec<entities::Task>, Error> {
        use entities::InboxReason;
        const ISSUE_FIELDS: &str =
            "number,title,state,repository,assignees,createdAt,updatedAt,url";
        const PR_FIELDS: &str =
            "number,title,state,repository,assignees,createdAt,updatedAt,url,isDraft";
        let login = self.my_login()?;

        let mut out = Vec::new();
        // Issues: anything I'm involved in, plus everything open in my repos.
        // `reason` records why each item is in the list; the more specific search
        // runs first so its reason wins over `Own`/`Involved` on dedup.
        for (args, reason) in [
            (
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
                InboxReason::Involved,
            ),
            (
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
                InboxReason::Own,
            ),
        ] {
            out.extend(parse_tasks(
                &self.runner.run(&args)?,
                TaskKind::Issue,
                reason,
            )?);
        }
        // PRs: review requested from me (highest-priority reason) first, then
        // involving me, then everything in my repos.
        for (args, reason) in [
            (
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
                InboxReason::ReviewRequested,
            ),
            (
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
                InboxReason::Involved,
            ),
            (
                vec![
                    "search", "prs", "--owner", &login, "--state", "open", "--json", PR_FIELDS,
                ],
                InboxReason::Own,
            ),
        ] {
            out.extend(parse_tasks(
                &self.runner.run(&args)?,
                TaskKind::PullRequest,
                reason,
            )?);
        }
        Ok(dedup_by_id(out))
    }

    /// My GitHub notifications, normalized. Fetches read ones too (`all=true`) so
    /// the dashboard can triage handled from pending; each item carries a `read`
    /// flag ([`notification_from_gh`]) for the client to filter on.
    pub fn list_notifications(&self) -> Result<Vec<entities::Notification>, Error> {
        let raw = self.runner.run(&["api", "notifications?all=true"])?;
        let arr: Vec<Value> = serde_json::from_slice(&raw)
            .map_err(|e| Error::Api(format!("parse gh output: {e}")))?;
        let mut items: Vec<entities::Notification> = arr.iter().map(notification_from_gh).collect();
        self.link_check_suite_runs(&mut items);
        Ok(items)
    }

    /// Upgrade each CI (CheckSuite) notification's link from the repository Actions
    /// page to the specific workflow run it refers to. The notification carries no
    /// run id, so the run is matched by the workflow name and branch parsed from
    /// the title and the run whose `updated_at` is closest to the notification's.
    /// Best-effort: a repository whose runs cannot be fetched keeps the
    /// Actions-page fallback, and a notification with no confident match is left
    /// unchanged.
    fn link_check_suite_runs(&self, items: &mut [entities::Notification]) {
        let is_ci = |i: &entities::Notification| i.kind == entities::NotifKind::CheckSuite;
        let repos: std::collections::BTreeSet<String> = items
            .iter()
            .filter(|i| is_ci(i))
            .filter_map(|i| i.project.clone())
            .collect();
        if repos.is_empty() {
            return;
        }
        let runs_by_repo: std::collections::HashMap<String, Vec<Value>> = repos
            .into_iter()
            .filter_map(|repo| self.fetch_recent_runs(&repo).map(|runs| (repo, runs)))
            .collect();
        for item in items.iter_mut() {
            if !is_ci(item) {
                continue;
            }
            let Some(runs) = item
                .project
                .as_deref()
                .and_then(|repo| runs_by_repo.get(repo))
            else {
                continue;
            };
            let updated = item.updated_at.as_deref().unwrap_or("");
            if let Some(url) = match_run_url(runs, &item.title, updated) {
                item.url = Some(url);
            }
        }
    }

    /// A repository's recent workflow runs (`workflow_runs` array), or `None` when
    /// the Actions API call or its parse fails (kept non-fatal so listing still
    /// succeeds with the Actions-page fallback).
    fn fetch_recent_runs(&self, repo: &str) -> Option<Vec<Value>> {
        let path = format!("repos/{repo}/actions/runs?per_page=100");
        let raw = self.runner.run(&["api", &path]).ok()?;
        let v: Value = serde_json::from_slice(&raw).ok()?;
        v.get("workflow_runs").and_then(Value::as_array).cloned()
    }

    /// Mark one notification thread as read (`PATCH /notifications/threads/{id}`).
    /// GitHub's notification list is unread-only, so a thread marked read simply
    /// drops from the next poll; there is no "mark unread" over the REST API.
    pub fn mark_notification_read(&self, id: i64) -> Result<(), Error> {
        let path = format!("notifications/threads/{id}");
        self.runner.run(&["api", "-X", "PATCH", &path])?;
        Ok(())
    }

    /// Mark every notification as read (`PUT /notifications`). Returns the number
    /// that were unread before the call, counted from the current list (GitHub's
    /// endpoint itself reports no count).
    pub fn mark_all_notifications_read(&self) -> Result<u64, Error> {
        let count = self.list_notifications()?.len() as u64;
        self.runner.run(&["api", "-X", "PUT", "notifications"])?;
        Ok(count)
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
                // `gh api user` (login lookup) vs `gh api notifications[?all=true]`.
                (Some("api"), Some("user")) => Ok(b"testuser".to_vec()),
                (Some("api"), Some(p)) if p.starts_with("notifications") => {
                    Ok(self.notifications.clone())
                }
                _ => Err(Error::Api(format!("unexpected gh args: {args:?}"))),
            }
        }
    }

    use std::cell::RefCell;
    use std::rc::Rc;

    /// Recording fake runner: returns the given notifications for
    /// `api notifications`, empty for any write, and records each invocation's
    /// joined args so a test can assert the exact endpoint and method.
    pub struct RecordGh {
        notifications: Vec<u8>,
        calls: Rc<RefCell<Vec<String>>>,
    }

    impl RecordGh {
        /// Returns the runner and a shared handle to inspect recorded calls after
        /// the runner has been moved into a backend.
        pub fn new(notifications: Vec<u8>) -> (Self, Rc<RefCell<Vec<String>>>) {
            let calls = Rc::new(RefCell::new(Vec::new()));
            (
                Self {
                    notifications,
                    calls: calls.clone(),
                },
                calls,
            )
        }
    }

    impl GhRunner for RecordGh {
        fn run(&self, args: &[&str]) -> Result<Vec<u8>, Error> {
            self.calls.borrow_mut().push(args.join(" "));
            match (args.first().copied(), args.get(1).copied()) {
                (Some("api"), Some(p)) if p.starts_with("notifications") => {
                    Ok(self.notifications.clone())
                }
                _ => Ok(Vec::new()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::tests_support::{FakeGh, RecordGh};
    use super::*;
    use serde_json::json;

    #[test]
    fn mark_notification_read_patches_the_thread() {
        let (gh, calls) = RecordGh::new(b"[]".to_vec());
        GithubBackend::new(gh).mark_notification_read(42).unwrap();
        assert_eq!(
            calls.borrow().as_slice(),
            &["api -X PATCH notifications/threads/42"]
        );
    }

    #[test]
    fn mark_all_read_counts_unread_then_puts() {
        let notifs = json!([
            {"id":"1","subject":{"title":"A"}},
            {"id":"2","subject":{"title":"B"}}
        ])
        .to_string()
        .into_bytes();
        let (gh, calls) = RecordGh::new(notifs);
        let count = GithubBackend::new(gh)
            .mark_all_notifications_read()
            .unwrap();
        assert_eq!(count, 2);
        // The list is read first (to count), then the mark-all PUT is issued.
        assert_eq!(
            calls.borrow().as_slice(),
            &["api notifications?all=true", "api -X PUT notifications"]
        );
    }

    #[test]
    fn list_notifications_requests_read_ones_too() {
        let (gh, calls) = RecordGh::new(b"[]".to_vec());
        GithubBackend::new(gh).list_notifications().unwrap();
        assert_eq!(calls.borrow().as_slice(), &["api notifications?all=true"]);
    }

    #[test]
    fn notification_from_gh_sets_read_flag_from_unread() {
        let read = notification_from_gh(&json!({
            "id": "1", "unread": false, "subject": {"title": "done"}
        }));
        assert!(read.read);
        let unread = notification_from_gh(&json!({
            "id": "2", "unread": true, "subject": {"title": "pending"}
        }));
        assert!(!unread.read);
        // A missing `unread` flag is treated as unread (read == false).
        let absent = notification_from_gh(&json!({
            "id": "3", "subject": {"title": "legacy"}
        }));
        assert!(!absent.read);
    }

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
    fn notification_from_gh_browser_url_for_pr_and_fallback() {
        // PullRequest maps to the `/pull/N` web path (not the API `/pulls/N`).
        let pr = json!({
            "subject": {"title": "PR", "type": "PullRequest", "url": "https://api.github.com/repos/acme/app/pulls/7"},
            "repository": {"full_name": "acme/app", "html_url": "https://github.com/acme/app"}
        });
        assert_eq!(
            notification_from_gh(&pr).url.as_deref(),
            Some("https://github.com/acme/app/pull/7")
        );
        // No html_url on the repository: fall back to github.com/<full_name>.
        let fallback = json!({
            "subject": {"title": "I", "type": "Issue", "url": "https://api.github.com/repos/acme/app/issues/3"},
            "repository": {"full_name": "acme/app"}
        });
        assert_eq!(
            notification_from_gh(&fallback).url.as_deref(),
            Some("https://github.com/acme/app/issues/3")
        );
        // Unsupported subject type: no link.
        let disc = json!({
            "subject": {"title": "D", "type": "Discussion", "url": "https://api.github.com/repos/acme/app/discussions/1"},
            "repository": {"full_name": "acme/app"}
        });
        assert_eq!(notification_from_gh(&disc).url, None);
    }

    #[test]
    fn notification_from_gh_check_suite_links_to_actions_page() {
        // CI (CheckSuite) notifications carry no subject number/url; they link to
        // the repository's Actions page (later upgraded to the specific run).
        let ci = json!({
            "subject": {"title": "CI workflow run failed", "type": "CheckSuite", "url": null},
            "repository": {"full_name": "acme/app", "html_url": "https://github.com/acme/app"}
        });
        assert_eq!(
            notification_from_gh(&ci).url.as_deref(),
            Some("https://github.com/acme/app/actions")
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
    fn task_from_gh_maps_to_the_typed_entity() {
        let v = json!({
            "number": 7, "title": "Fix gearbox", "state": "open",
            "repository": {"nameWithOwner": "acme/widgets"},
            "assignees": [{"login": "dana"}, {"login": "robin"}],
            "createdAt": "2026-07-01T09:00:00Z", "updatedAt": "2026-07-10T12:00:00Z",
            "url": "https://example.test/acme/widgets/pull/7"
        });
        let t = task_from_gh(
            &v,
            TaskKind::PullRequest,
            entities::InboxReason::ReviewRequested,
        );
        assert_eq!(t.id.display, "acme/widgets#7");
        assert_eq!(t.id.raw, "7");
        assert_eq!(t.kind, entities::TaskKind::PullRequest);
        assert_eq!(t.reason, entities::InboxReason::ReviewRequested);
        assert_eq!(t.title, "Fix gearbox");
        assert_eq!(t.status.as_deref(), Some("open"));
        assert_eq!(t.status_category, entities::StatusCategory::Open);
        assert_eq!(t.project.as_deref(), Some("acme/widgets"));
        assert_eq!(t.assignee.as_deref(), Some("dana, robin"));
        assert_eq!(
            t.url.as_deref(),
            Some("https://example.test/acme/widgets/pull/7")
        );
    }

    #[test]
    fn notification_from_gh_maps_check_suite_with_outcome_and_browser_url() {
        let v = json!({
            "id": "42", "reason": "ci_activity", "unread": true,
            "subject": {"title": "CI workflow run failed for main branch", "type": "CheckSuite"},
            "repository": {"full_name": "acme/widgets"},
            "updated_at": "2026-07-10T12:00:00Z"
        });
        let n = notification_from_gh(&v);
        assert_eq!(n.id, "42");
        assert_eq!(n.reason, "ci_activity");
        assert_eq!(n.kind, entities::NotifKind::CheckSuite);
        assert_eq!(n.outcome, Some(entities::CiOutcome::Failure));
        assert!(!n.read);
        assert_eq!(n.wp_id, None);
        // Issue notification: browser url derived from the subject number, read flag set.
        let issue = json!({
            "id": "9", "reason": "mention", "unread": false,
            "subject": {"title": "Ping", "type": "Issue",
                "url": "https://api.github.com/repos/acme/widgets/issues/3"},
            "repository": {"full_name": "acme/widgets"}
        });
        let n2 = notification_from_gh(&issue);
        assert_eq!(n2.kind, entities::NotifKind::Issue);
        assert!(n2.read);
        assert_eq!(n2.outcome, None);
        assert_eq!(
            n2.url.as_deref(),
            Some("https://github.com/acme/widgets/issues/3")
        );
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
        assert_eq!(out[0].kind, entities::TaskKind::Issue);
        assert_eq!(out[0].id.display, "acme/app#1");
        assert_eq!(out[1].kind, entities::TaskKind::PullRequest);
        assert_eq!(out[1].id.display, "acme/app#2");
        // The PR surfaces first from the review-requested search, so its reason wins.
        assert_eq!(out[1].reason, entities::InboxReason::ReviewRequested);
    }

    /// Build a minimal typed task with the given display id and reason.
    fn task(display: &str, reason: entities::InboxReason) -> entities::Task {
        entities::Task {
            id: entities::TaskId {
                display: display.into(),
                raw: display.into(),
            },
            kind: entities::TaskKind::Issue,
            reason,
            title: String::new(),
            url: None,
            status: None,
            status_category: entities::StatusCategory::Unknown,
            project: None,
            assignee: None,
            author: None,
            created_at: None,
            updated_at: None,
            due_date: None,
            priority: None,
            labels: Vec::new(),
            custom_fields: Vec::new(),
        }
    }

    #[test]
    fn dedup_by_id_keeps_first_occurrence_and_order() {
        let a1 = task("acme/app#1", entities::InboxReason::ReviewRequested);
        let a1_dup = task("acme/app#1", entities::InboxReason::Involved);
        let b2 = task("acme/app#2", entities::InboxReason::Own);
        let out = dedup_by_id(vec![a1, b2, a1_dup]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].id.display, "acme/app#1");
        // The first occurrence (ReviewRequested) is kept over the later duplicate.
        assert_eq!(out[0].reason, entities::InboxReason::ReviewRequested);
        assert_eq!(out[1].id.display, "acme/app#2");
    }

    #[test]
    fn parses_workflow_and_branch_from_check_suite_title() {
        let t = "CI workflow run failed for deps/keyring-core branch";
        assert_eq!(check_suite_workflow(t), Some("CI"));
        assert_eq!(check_suite_branch(t), Some("deps/keyring-core".to_string()));
        // A title without the marker yields no workflow / no branch.
        assert_eq!(check_suite_workflow("Deploy done"), None);
        assert_eq!(check_suite_branch("no branch marker here"), None);
    }

    #[test]
    fn match_run_url_picks_workflow_branch_and_nearest_time() {
        let runs = json!([
            {"name":"CI","head_branch":"feat","updated_at":"2026-07-15T11:21:20Z",
             "html_url":"https://github.com/acme/app/actions/runs/999"},
            {"name":"CI","head_branch":"master","updated_at":"2026-07-15T11:21:19Z",
             "html_url":"https://github.com/acme/app/actions/runs/111"},
            {"name":"Audit","head_branch":"feat","updated_at":"2026-07-15T11:21:18Z",
             "html_url":"https://github.com/acme/app/actions/runs/222"},
            {"name":"CI","head_branch":"feat","updated_at":"2026-07-10T00:00:00Z",
             "html_url":"https://github.com/acme/app/actions/runs/333"}
        ]);
        let runs = runs.as_array().unwrap();
        // Workflow "CI" + branch "feat", nearest to 11:21:18 → run 999 (not the
        // wrong branch 111, wrong workflow 222, or the far-older 333).
        assert_eq!(
            match_run_url(
                runs,
                "CI workflow run failed for feat branch",
                "2026-07-15T11:21:18Z"
            ),
            Some("https://github.com/acme/app/actions/runs/999".to_string())
        );
        // No run for that workflow → no match.
        assert_eq!(
            match_run_url(
                runs,
                "Release workflow run failed for feat branch",
                "2026-07-15T11:21:18Z"
            ),
            None
        );
    }

    /// Fake runner serving both the notifications inbox and a repository's runs.
    struct CiGh {
        notifs: Vec<u8>,
        runs: Result<Vec<u8>, ()>,
    }
    impl GhRunner for CiGh {
        fn run(&self, args: &[&str]) -> Result<Vec<u8>, Error> {
            match (args.first().copied(), args.get(1).copied()) {
                (Some("api"), Some(p)) if p.starts_with("notifications") => Ok(self.notifs.clone()),
                (Some("api"), Some(p)) if p.contains("/actions/runs") => self
                    .runs
                    .clone()
                    .map_err(|()| Error::Api("runs unavailable".into())),
                _ => Err(Error::Api(format!("unexpected gh args: {args:?}"))),
            }
        }
    }

    fn ci_notification() -> Vec<u8> {
        json!([{
            "id":"1","reason":"ci_activity",
            "subject":{"title":"CI workflow run failed for deps/keyring-core branch",
                       "type":"CheckSuite","url":null},
            "repository":{"full_name":"acme/app","html_url":"https://github.com/acme/app"},
            "updated_at":"2026-07-15T11:21:18Z"
        }])
        .to_string()
        .into_bytes()
    }

    #[test]
    fn list_notifications_links_check_suite_to_specific_run() {
        let runs = json!({"workflow_runs":[
            {"name":"CI","head_branch":"deps/keyring-core","updated_at":"2026-07-15T11:21:20Z",
             "html_url":"https://github.com/acme/app/actions/runs/999"},
            {"name":"CI","head_branch":"master","updated_at":"2026-07-15T11:21:19Z",
             "html_url":"https://github.com/acme/app/actions/runs/111"}
        ]})
        .to_string()
        .into_bytes();
        let gh = CiGh {
            notifs: ci_notification(),
            runs: Ok(runs),
        };
        let out = GithubBackend::new(gh).list_notifications().unwrap();
        assert_eq!(
            out[0].url.as_deref(),
            Some("https://github.com/acme/app/actions/runs/999")
        );
    }

    #[test]
    fn check_suite_keeps_actions_fallback_when_runs_unavailable() {
        let gh = CiGh {
            notifs: ci_notification(),
            runs: Err(()),
        };
        let out = GithubBackend::new(gh).list_notifications().unwrap();
        // Runs could not be fetched → the link stays the repository Actions page.
        assert_eq!(
            out[0].url.as_deref(),
            Some("https://github.com/acme/app/actions")
        );
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
        assert_eq!(out[0].title, "T");
        assert_eq!(out[0].project.as_deref(), Some("acme/app"));
    }
}
