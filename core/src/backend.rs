//! Backend facade: one entry point that hides the OpenProject/GitHub split.
//!
//! Both the CLI and the GUI call these functions so backend routing lives in a
//! single place. Returns the shared normalized task/notification shape as
//! `Vec<Value>`.

use serde_json::Value;

use crate::client::Client;
use crate::config::{Backend, ServerProfile};
use crate::error::Error;
use crate::github::{GhCli, GhRunner, GithubBackend};
use crate::resources::{notification, work_packages};

/// My open tasks for a server, normalized. OpenProject requires a token; GitHub
/// authenticates through `gh` and ignores it.
pub async fn list_tasks(profile: &ServerProfile, token: Option<&str>) -> Result<Vec<Value>, Error> {
    match profile.backend {
        Backend::OpenProject => {
            let client = openproject_client(profile, token)?;
            let params = work_packages::WpListParams {
                assignee: Some("me".into()),
                open: true,
                offset: 1,
                ..Default::default()
            };
            Ok(as_array(work_packages::list(&client, params, false).await?))
        }
        Backend::Github => {
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
        Backend::OpenProject => {
            let client = openproject_client(profile, token)?;
            Ok(as_array(notification::list(&client, 1, None, false).await?))
        }
        Backend::Github => {
            let host = profile.base_url.clone();
            run_blocking(move || github_notifications(GhCli { host })).await
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

    #[tokio::test]
    async fn openproject_tasks_require_token() {
        let p = ServerProfile {
            base_url: "https://op.example".into(),
            backend: Backend::OpenProject,
            timeout: 30,
            verify_ssl: true,
            proxy: None,
        };
        let err = list_tasks(&p, None).await.unwrap_err();
        assert!(matches!(err, Error::Auth(_)));
    }
}
