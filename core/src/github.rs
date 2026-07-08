//! GitHub backend (read-only) driven through the `gh` CLI.
//!
//! First slice: list my open issues and pull requests, and my notifications.
//! `gh` handles authentication and host selection, so no token is stored here.
//! Output is normalized to the same shape as an OpenProject work package so the
//! CLI and GUI can render both backends uniformly.

use serde_json::{Map, Value};

use crate::error::Error;

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
    Value::Object(m)
}

fn parse_tasks(raw: &[u8], kind: TaskKind) -> Result<Vec<Value>, Error> {
    let arr: Vec<Value> =
        serde_json::from_slice(raw).map_err(|e| Error::Api(format!("parse gh output: {e}")))?;
    Ok(arr.iter().map(|v| normalize_task(v, kind)).collect())
}

/// Read-only GitHub backend over a [`GhRunner`].
pub struct GithubBackend<R: GhRunner> {
    runner: R,
}

impl<R: GhRunner> GithubBackend<R> {
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    /// My open issues and pull requests (aggregated), normalized.
    pub fn list_my_tasks(&self) -> Result<Vec<Value>, Error> {
        let issues = self.runner.run(&[
            "search",
            "issues",
            "--assignee",
            "@me",
            "--state",
            "open",
            "--json",
            "number,title,state,repository,assignees,createdAt,updatedAt,url",
        ])?;
        let prs = self.runner.run(&[
            "search",
            "prs",
            "--assignee",
            "@me",
            "--state",
            "open",
            "--json",
            "number,title,state,repository,assignees,createdAt,updatedAt,url,isDraft",
        ])?;
        let mut out = parse_tasks(&issues, TaskKind::Issue)?;
        out.extend(parse_tasks(&prs, TaskKind::PullRequest)?);
        Ok(out)
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
                (Some("api"), _) => Ok(self.notifications.clone()),
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
            "repository": {"full_name": "acme/app"},
            "updated_at": "2026-07-02T00:00:00Z"
        });
        let n = normalize_notification(&v);
        assert_eq!(n["id"], json!("9001"));
        assert_eq!(n["reason"], json!("mention"));
        assert_eq!(n["subject"], json!("Ping"));
        assert_eq!(n["type"], json!("Issue"));
        assert_eq!(n["project"], json!("acme/app"));
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
        let out = GithubBackend::new(fake).list_my_tasks().unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["type"], json!("Issue"));
        assert_eq!(out[0]["id"], json!("acme/app#1"));
        assert_eq!(out[1]["type"], json!("Pull request"));
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
