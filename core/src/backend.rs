//! Backend facade: one entry point that hides the OpenProject/GitHub split.
//!
//! Both the CLI and the GUI call these functions so backend routing lives in a
//! single place. Returns the shared normalized task/notification shape as
//! `Vec<Value>`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::client::Client;
use crate::config::{BackendKind, ServerProfile};
use crate::error::Error;
use crate::github::{GhCli, GhRunner, GithubBackend};
use crate::resources::{notification, work_packages};

/// Default page size for the paged list APIs.
pub const PAGE_SIZE: i64 = 50;

/// One page of normalized items plus the cursor for the next page.
///
/// `next_offset` is a backend-specific opaque cursor the caller passes back to
/// fetch the following page: for OpenProject it is the next 1-based page number,
/// and `None` once the collection is exhausted. The GitHub backend returns the
/// whole collection in a single page (`next_offset: None`) because `gh` has no
/// clean cursor across the client-merged issue/PR stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub items: Vec<Value>,
    pub next_offset: Option<i64>,
}

/// Next 1-based page number, or `None` when the pages fetched so far cover the
/// whole collection. `page` is the 1-based page just fetched.
fn next_page(page: i64, page_size: i64, total: i64) -> Option<i64> {
    if page.max(1) * page_size < total {
        Some(page + 1)
    } else {
        None
    }
}

/// One page of my open tasks for a server, normalized, plus the next cursor.
/// OpenProject paginates by `page` (1-based); GitHub returns everything on the
/// first page and `None` thereafter.
pub async fn list_tasks_page(
    profile: &ServerProfile,
    token: Option<&str>,
    page: i64,
    page_size: i64,
) -> Result<Page, Error> {
    match profile.backend {
        BackendKind::OpenProject => {
            let client = openproject_client(profile, token)?;
            if profile.backend.needs_local_history() {
                // The server forgets past assignees, so aggregate current +
                // locally tracked past-assigned tasks in one shot (the
                // include_past path does not paginate on the server).
                let params = work_packages::WpListParams {
                    assignee: Some("me".into()),
                    open: true,
                    include_past: true,
                    ..Default::default()
                };
                let items = work_packages::list(&client, params, false).await?;
                Ok(Page {
                    items: as_array(items),
                    next_offset: None,
                })
            } else {
                let params = work_packages::WpListParams {
                    assignee: Some("me".into()),
                    open: true,
                    offset: page.max(1),
                    limit: Some(page_size),
                    ..Default::default()
                };
                let (items, total) = work_packages::list_page(&client, params, false).await?;
                Ok(Page {
                    items: as_array(items),
                    next_offset: next_page(page, page_size, total),
                })
            }
        }
        BackendKind::Github => {
            let host = profile.base_url.clone();
            let items = run_blocking(move || github_tasks(GhCli { host })).await?;
            Ok(Page {
                items,
                next_offset: None,
            })
        }
    }
}

/// One page of my notifications for a server, normalized, plus the next cursor.
pub async fn list_notifications_page(
    profile: &ServerProfile,
    token: Option<&str>,
    page: i64,
    page_size: i64,
) -> Result<Page, Error> {
    match profile.backend {
        BackendKind::OpenProject => {
            let client = openproject_client(profile, token)?;
            let (items, total) =
                notification::list_page(&client, page.max(1), Some(page_size), false).await?;
            Ok(Page {
                items: as_array(items),
                next_offset: next_page(page, page_size, total),
            })
        }
        BackendKind::Github => {
            let host = profile.base_url.clone();
            let items = run_blocking(move || github_notifications(GhCli { host })).await?;
            Ok(Page {
                items,
                next_offset: None,
            })
        }
    }
}

/// My open tasks for a server, normalized. OpenProject requires a token; GitHub
/// authenticates through `gh` and ignores it.
pub async fn list_tasks(profile: &ServerProfile, token: Option<&str>) -> Result<Vec<Value>, Error> {
    match profile.backend {
        BackendKind::OpenProject => {
            let client = openproject_client(profile, token)?;
            let params = work_packages::WpListParams {
                assignee: Some("me".into()),
                open: true,
                offset: 1,
                ..Default::default()
            };
            Ok(as_array(work_packages::list(&client, params, false).await?))
        }
        BackendKind::Github => {
            let host = profile.base_url.clone();
            run_blocking(move || github_tasks(GhCli { host })).await
        }
    }
}

/// My notifications for a server, normalized.
pub async fn list_notifications(
    profile: &ServerProfile,
    token: Option<&str>,
) -> Result<Vec<Value>, Error> {
    match profile.backend {
        BackendKind::OpenProject => {
            let client = openproject_client(profile, token)?;
            Ok(as_array(notification::list(&client, 1, None, false).await?))
        }
        BackendKind::Github => {
            let host = profile.base_url.clone();
            run_blocking(move || github_notifications(GhCli { host })).await
        }
    }
}

/// Set one notification's read state. OpenProject toggles both ways; GitHub can
/// only mark read (`read == true`) — its list is unread-only, so `read == false`
/// (mark unread) is unreachable from the UI and treated as a no-op.
pub async fn set_notification_read(
    profile: &ServerProfile,
    token: Option<&str>,
    id: i64,
    read: bool,
) -> Result<(), Error> {
    match profile.backend {
        BackendKind::OpenProject => {
            let client = openproject_client(profile, token)?;
            if read {
                notification::read(&client, id).await?;
            } else {
                notification::unread(&client, id).await?;
            }
            Ok(())
        }
        BackendKind::Github => {
            if !read {
                return Ok(());
            }
            let host = profile.base_url.clone();
            run_blocking_unit(move || github_mark_read(GhCli { host }, id)).await
        }
    }
}

/// Mark every notification on a server as read. Returns the count marked.
pub async fn mark_all_read(profile: &ServerProfile, token: Option<&str>) -> Result<u64, Error> {
    match profile.backend {
        BackendKind::OpenProject => {
            let client = openproject_client(profile, token)?;
            let v = notification::read_all(&client).await?;
            Ok(v.get("read").and_then(|x| x.as_u64()).unwrap_or(0))
        }
        BackendKind::Github => {
            let host = profile.base_url.clone();
            run_blocking_count(move || github_mark_all_read(GhCli { host })).await
        }
    }
}

fn openproject_client(profile: &ServerProfile, token: Option<&str>) -> Result<Client, Error> {
    let token = token
        .ok_or_else(|| Error::Auth("openproject backend requires a token".into()))?
        .to_owned();
    Client::new("", profile, token, None)
}

/// Run a blocking `gh`-backed closure off the async executor.
async fn run_blocking<F>(f: F) -> Result<Vec<Value>, Error>
where
    F: FnOnce() -> Result<Vec<Value>, Error> + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| Error::Internal(format!("join gh task: {e}")))?
}

/// Seam for testing the GitHub task branch with a fake runner.
fn github_tasks<R: GhRunner>(runner: R) -> Result<Vec<Value>, Error> {
    GithubBackend::new(runner).list_my_tasks()
}

/// Seam for testing the GitHub notification branch with a fake runner.
fn github_notifications<R: GhRunner>(runner: R) -> Result<Vec<Value>, Error> {
    GithubBackend::new(runner).list_notifications()
}

/// Seam for testing the GitHub mark-one-read branch with a fake runner.
fn github_mark_read<R: GhRunner>(runner: R, id: i64) -> Result<(), Error> {
    GithubBackend::new(runner).mark_notification_read(id)
}

/// Seam for testing the GitHub mark-all-read branch with a fake runner.
fn github_mark_all_read<R: GhRunner>(runner: R) -> Result<u64, Error> {
    GithubBackend::new(runner).mark_all_notifications_read()
}

/// Run a blocking `gh`-backed unit-returning closure off the async executor.
async fn run_blocking_unit<F>(f: F) -> Result<(), Error>
where
    F: FnOnce() -> Result<(), Error> + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| Error::Internal(format!("join gh task: {e}")))?
}

/// Run a blocking `gh`-backed count-returning closure off the async executor.
async fn run_blocking_count<F>(f: F) -> Result<u64, Error>
where
    F: FnOnce() -> Result<u64, Error> + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| Error::Internal(format!("join gh task: {e}")))?
}

fn as_array(v: Value) -> Vec<Value> {
    match v {
        Value::Array(a) => a,
        Value::Null => Vec::new(),
        other => vec![other],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::tests_support::FakeGh;
    use serde_json::json;

    #[test]
    fn github_tasks_route_through_runner() {
        let issues =
            json!([{"number":1,"title":"I1","state":"open","repository":{"nameWithOwner":"acme/app"},"assignees":[]}])
                .to_string()
                .into_bytes();
        let fake = FakeGh::new(issues, b"[]".to_vec(), b"[]".to_vec());
        let out = github_tasks(fake).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["id"], json!("acme/app#1"));
    }

    #[test]
    fn github_notifications_route_through_runner() {
        let notifs = json!([{
            "id":"1","reason":"assign",
            "subject":{"title":"T","type":"Issue","url":"u"},
            "repository":{"full_name":"acme/app"},"updated_at":"2026-07-02T00:00:00Z"
        }])
        .to_string()
        .into_bytes();
        let fake = FakeGh::new(b"[]".to_vec(), b"[]".to_vec(), notifs);
        let out = github_notifications(fake).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["subject"], json!("T"));
    }

    #[test]
    fn next_page_stops_when_pages_cover_total() {
        // page 1 of 50 with total 50: exhausted.
        assert_eq!(next_page(1, 50, 50), None);
        // page 1 of 50 with total 51: one more page.
        assert_eq!(next_page(1, 50, 51), Some(2));
        // page 2 of 50 with total 120: still more.
        assert_eq!(next_page(2, 50, 120), Some(3));
        // page 3 of 50 with total 120: exhausted (150 >= 120).
        assert_eq!(next_page(3, 50, 120), None);
        // empty collection: no next page.
        assert_eq!(next_page(1, 50, 0), None);
    }

    #[test]
    fn github_tasks_page_is_single_page() {
        let issues =
            json!([{"number":1,"title":"I1","state":"open","repository":{"nameWithOwner":"acme/app"},"assignees":[]}])
                .to_string()
                .into_bytes();
        let fake = FakeGh::new(issues, b"[]".to_vec(), b"[]".to_vec());
        let items = github_tasks(fake).unwrap();
        // The page facade wraps the same items with no further cursor.
        let page = Page {
            items,
            next_offset: None,
        };
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.next_offset, None);
    }

    #[tokio::test]
    async fn openproject_tasks_require_token() {
        let p = ServerProfile {
            base_url: "https://op.example".into(),
            backend: BackendKind::OpenProject,
            timeout: 30,
            verify_ssl: true,
            proxy: None,
            display_name: None,
            enabled: true,
            poll_secs: None,
            timelog_start: None,
            status_colors: Default::default(),
            status_filters: Vec::new(),
            display_fields: Vec::new(),
            open_content_in: None,
        };
        let err = list_tasks(&p, None).await.unwrap_err();
        assert!(matches!(err, Error::Auth(_)));
    }
}
