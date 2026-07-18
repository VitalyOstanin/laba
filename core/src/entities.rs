//! Typed domain entities shared across backends.
//!
//! These replace the untyped `serde_json::Value` "shared shape" that tasks and
//! notifications used to flow through: each backend fills the same struct, so a
//! field a backend forgets to set is a compile error rather than a silent gap
//! (the precedent being GitHub notifications lacking a `read` flag until it was
//! noticed at runtime). The form is deliberately backend-neutral — OpenProject
//! is one implementation, not the template.
//!
//! Serialized camelCase for the GUI, which consumes real `interface Task` /
//! `Notification` types instead of open-ended records.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

/// A task's identity: a human-facing `display` string (shown in the list, format
/// varies by backend — `owner/repo#7`, `PROJ-45`, `#123`) and the `raw` id a
/// backend uses for its own API calls (a number, an issue key, a node id).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskId {
    pub display: String,
    pub raw: String,
}

/// What kind of thing a task is. `Other` keeps the enum open so a new backend's
/// task type does not force a variant here; the string is the backend's own label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskKind {
    Issue,
    PullRequest,
    WorkPackage,
    Other(String),
}

impl TaskKind {
    /// The wire token (`issue` / `pullRequest` / `workPackage`, or the raw label
    /// for `Other`). Also the value the GUI matches on.
    pub fn as_token(&self) -> String {
        match self {
            TaskKind::Issue => "issue".into(),
            TaskKind::PullRequest => "pullRequest".into(),
            TaskKind::WorkPackage => "workPackage".into(),
            TaskKind::Other(s) => s.clone(),
        }
    }

    /// Parse a wire token back to a variant; anything unrecognized becomes `Other`.
    pub fn from_token(s: &str) -> TaskKind {
        match s {
            "issue" => TaskKind::Issue,
            "pullRequest" => TaskKind::PullRequest,
            "workPackage" => TaskKind::WorkPackage,
            other => TaskKind::Other(other.to_owned()),
        }
    }
}

/// Why a task is in the user's list — the core of "don't miss it": a pull request
/// awaiting review reads differently from an assigned task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InboxReason {
    Assigned,
    Authored,
    ReviewRequested,
    Mentioned,
    Involved,
    Own,
}

/// Normalized workflow status bucket, for tinting and filtering across backends
/// whose raw status labels differ. `Unknown` when the raw status maps to none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StatusCategory {
    Open,
    InProgress,
    Done,
    Unknown,
}

/// One backend-specific extra field, shown as an optional task-list column. `name`
/// is the human label (also the column header and the display-field match key).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomField {
    pub key: String,
    pub name: Option<String>,
    pub value: Value,
}

/// A task (issue / pull request / work package) in the user's inbox. Every field
/// a backend cannot supply is `None` / empty rather than omitted, so the GUI
/// contract is stable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: TaskId,
    pub kind: TaskKind,
    pub reason: InboxReason,
    pub title: String,
    /// Browser URL for the task, when the backend exposes one.
    pub url: Option<String>,
    /// Raw workflow status label as the backend reports it (e.g. "In progress").
    pub status: Option<String>,
    /// Normalized status bucket derived from `status`, for tint and filter.
    pub status_category: StatusCategory,
    /// Repository / project the task belongs to.
    pub project: Option<String>,
    /// True when the task lives in a repository the user owns (GitHub: the
    /// repository owner equals the authenticated login). Drives the "My repos"
    /// vs "All" scope tabs. `#[serde(default)]` keeps older payloads loadable.
    #[serde(default)]
    pub mine: bool,
    pub assignee: Option<String>,
    pub author: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub due_date: Option<String>,
    /// Priority label (Jira / YouTrack); backends without a priority leave it `None`.
    pub priority: Option<String>,
    pub labels: Vec<String>,
    /// Backend-specific extra fields the user may show as columns.
    pub custom_fields: Vec<CustomField>,
}

/// What a notification is about. `Other` keeps it open for backend-specific
/// subject types (Discussion, Release, …) without a variant per case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotifKind {
    Issue,
    PullRequest,
    CheckSuite,
    Other(String),
}

impl NotifKind {
    /// The wire token (`issue` / `pullRequest` / `checkSuite`, or the raw label).
    pub fn as_token(&self) -> String {
        match self {
            NotifKind::Issue => "issue".into(),
            NotifKind::PullRequest => "pullRequest".into(),
            NotifKind::CheckSuite => "checkSuite".into(),
            NotifKind::Other(s) => s.clone(),
        }
    }

    /// Parse a wire token; anything unrecognized becomes `Other`.
    pub fn from_token(s: &str) -> NotifKind {
        match s {
            "issue" => NotifKind::Issue,
            "pullRequest" => NotifKind::PullRequest,
            "checkSuite" => NotifKind::CheckSuite,
            other => NotifKind::Other(other.to_owned()),
        }
    }
}

/// The outcome of a CI (check-suite) notification, for tinting: a failed run
/// reads as a warning, a successful run as good.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CiOutcome {
    Failure,
    Success,
    Neutral,
}

/// A notification in the user's inbox. `url` is resolved to a concrete target
/// where possible (including a specific CI run), so clicking goes to the right
/// place rather than a generic page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    /// Thread / event id (the handle used to mark it read).
    pub id: String,
    /// Backend reason string (`ci_activity`, `mention`, `assign`, …).
    pub reason: String,
    pub kind: NotifKind,
    pub title: String,
    pub project: Option<String>,
    /// Browser URL for the notification's subject, when one can be built.
    pub url: Option<String>,
    pub updated_at: Option<String>,
    pub read: bool,
    /// CI run outcome, present only for check-suite notifications.
    pub outcome: Option<CiOutcome>,
    /// The related work package id, when the notification points at one and the
    /// backend can open it in-app (OpenProject). `None` otherwise.
    pub wp_id: Option<i64>,
}

/// A task opened for its full description and comment thread (the detail screen).
/// Only backends with `DetailSupport::InApp` produce one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDetail {
    pub task: Task,
    pub description: Option<String>,
    pub comments: Vec<Comment>,
}

/// One comment on a task's detail screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub author: Option<String>,
    pub created_at: Option<String>,
    pub body: String,
}

// TaskKind / NotifKind serialize as their flat wire token (a plain string), with
// `Other` round-tripping through the raw label. A manual impl keeps `Other`
// transparent on the wire instead of serde's `{ "other": "..." }` tagging.
impl Serialize for TaskKind {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.as_token())
    }
}
impl<'de> Deserialize<'de> for TaskKind {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(TaskKind::from_token(&String::deserialize(d)?))
    }
}
impl Serialize for NotifKind {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.as_token())
    }
}
impl<'de> Deserialize<'de> for NotifKind {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(NotifKind::from_token(&String::deserialize(d)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn task_serializes_to_the_camelcase_gui_contract() {
        let task = Task {
            id: TaskId {
                display: "acme/widgets#7".into(),
                raw: "7".into(),
            },
            kind: TaskKind::PullRequest,
            reason: InboxReason::ReviewRequested,
            title: "Fix the gearbox".into(),
            url: Some("https://example.test/acme/widgets/pull/7".into()),
            status: Some("open".into()),
            status_category: StatusCategory::Open,
            project: Some("acme/widgets".into()),
            mine: true,
            assignee: Some("dana".into()),
            author: Some("robin".into()),
            created_at: Some("2026-07-01T09:00:00Z".into()),
            updated_at: Some("2026-07-10T12:00:00Z".into()),
            due_date: None,
            priority: None,
            labels: vec!["bug".into()],
            custom_fields: vec![CustomField {
                key: "rank".into(),
                name: Some("Rank".into()),
                value: json!("1|hzz"),
            }],
        };
        assert_eq!(
            serde_json::to_value(&task).unwrap(),
            json!({
                "id": {"display": "acme/widgets#7", "raw": "7"},
                "kind": "pullRequest",
                "reason": "reviewRequested",
                "title": "Fix the gearbox",
                "url": "https://example.test/acme/widgets/pull/7",
                "status": "open",
                "statusCategory": "open",
                "project": "acme/widgets",
                "mine": true,
                "assignee": "dana",
                "author": "robin",
                "createdAt": "2026-07-01T09:00:00Z",
                "updatedAt": "2026-07-10T12:00:00Z",
                "dueDate": null,
                "priority": null,
                "labels": ["bug"],
                "customFields": [{"key": "rank", "name": "Rank", "value": "1|hzz"}],
            })
        );
    }

    #[test]
    fn notification_serializes_to_the_camelcase_gui_contract() {
        let n = Notification {
            id: "42".into(),
            reason: "ci_activity".into(),
            kind: NotifKind::CheckSuite,
            title: "CI workflow run failed for main branch".into(),
            project: Some("acme/widgets".into()),
            url: Some("https://example.test/acme/widgets/actions/runs/9".into()),
            updated_at: Some("2026-07-10T12:00:00Z".into()),
            read: false,
            outcome: Some(CiOutcome::Failure),
            wp_id: None,
        };
        assert_eq!(
            serde_json::to_value(&n).unwrap(),
            json!({
                "id": "42",
                "reason": "ci_activity",
                "kind": "checkSuite",
                "title": "CI workflow run failed for main branch",
                "project": "acme/widgets",
                "url": "https://example.test/acme/widgets/actions/runs/9",
                "updatedAt": "2026-07-10T12:00:00Z",
                "read": false,
                "outcome": "failure",
                "wpId": null,
            })
        );
    }

    #[test]
    fn open_kinds_round_trip_through_their_raw_label() {
        for tok in ["issue", "pullRequest", "workPackage", "discussion"] {
            let k = TaskKind::from_token(tok);
            assert_eq!(k.as_token(), tok);
            assert_eq!(serde_json::to_value(&k).unwrap(), Value::String(tok.into()));
            assert_eq!(serde_json::from_value::<TaskKind>(json!(tok)).unwrap(), k);
        }
        assert_eq!(
            NotifKind::from_token("Release"),
            NotifKind::Other("Release".into())
        );
        assert_eq!(NotifKind::Other("Release".into()).as_token(), "Release");
    }

    #[test]
    fn task_detail_serializes_nested_task_and_comments() {
        let detail = TaskDetail {
            task: Task {
                id: TaskId {
                    display: "#1".into(),
                    raw: "1".into(),
                },
                kind: TaskKind::WorkPackage,
                reason: InboxReason::Assigned,
                title: "Do the thing".into(),
                url: None,
                status: Some("New".into()),
                status_category: StatusCategory::Open,
                project: Some("Ops".into()),
                mine: false,
                assignee: None,
                author: None,
                created_at: None,
                updated_at: None,
                due_date: None,
                priority: None,
                labels: vec![],
                custom_fields: vec![],
            },
            description: Some("Body text".into()),
            comments: vec![Comment {
                author: Some("dana".into()),
                created_at: Some("2026-07-10T12:00:00Z".into()),
                body: "Looks good".into(),
            }],
        };
        let v = serde_json::to_value(&detail).unwrap();
        assert_eq!(v["task"]["kind"], json!("workPackage"));
        assert_eq!(v["description"], json!("Body text"));
        assert_eq!(v["comments"][0]["author"], json!("dana"));
        assert_eq!(v["comments"][0]["createdAt"], json!("2026-07-10T12:00:00Z"));
    }
}
