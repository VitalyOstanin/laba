//! Reference resolvers: turn human-friendly references (identifiers, names,
//! principal logins) into numeric ids expected by the API. Enumeration lookups
//! (`Status`, `Type`, `Activity`) are cached per server. Error texts mirror the
//! predecessor Python tool for output parity.

use serde_json::json;

use crate::client::Client;
use crate::error::Error;

/// True when `s` is non-empty and every character is an ASCII digit.
fn is_ascii_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// Render an element's `id` (number or string) as a plain string.
fn id_to_string(v: &serde_json::Value) -> Option<String> {
    match v.get("id") {
        Some(serde_json::Value::Number(n)) => Some(n.to_string()),
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

/// Resolve a project reference (numeric id or identifier) to its numeric id.
/// The `projects/{ref}` endpoint accepts either form.
pub async fn project_id(client: &Client, ref_: &str) -> Result<String, Error> {
    let ref_ = ref_.trim();
    let payload = client
        .request_json(reqwest::Method::GET, &format!("projects/{ref_}"), None)
        .await?;
    match payload.get("id") {
        Some(serde_json::Value::Number(n)) => Ok(n.to_string()),
        Some(serde_json::Value::String(s)) => Ok(s.clone()),
        _ => Err(Error::Api(format!("Project {ref_:?} was not found."))),
    }
}

/// Resolve an enumeration entry by numeric id (passthrough) or by name
/// (case-insensitive), collecting the whole enumeration and caching the hit.
async fn resolve_by_name(
    client: &Client,
    path: &str,
    ref_: &str,
    kind: &str,
) -> Result<String, Error> {
    let ref_ = ref_.trim();
    if is_ascii_digits(ref_) {
        return Ok(ref_.to_string());
    }
    if let Some(id) = client.cache().get_resolve(kind, ref_) {
        return Ok(id);
    }
    let elements = client.collect(path, &[]).await?;
    let target = ref_.to_lowercase();
    let matches: Vec<&serde_json::Value> = elements
        .iter()
        .filter(|e| {
            e.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n.to_lowercase() == target)
                .unwrap_or(false)
        })
        .collect();
    match matches.len() {
        0 => {
            let mut names: Vec<String> = elements
                .iter()
                .filter_map(|e| e.get("name").and_then(|n| n.as_str()).map(str::to_owned))
                .collect();
            names.sort();
            let names = names.join(", ");
            Err(Error::Api(format!(
                "{kind} {ref_:?} was not found. Available: {names}"
            )))
        }
        1 => {
            let id = id_to_string(matches[0])
                .ok_or_else(|| Error::Api(format!("{kind} {ref_:?} has no id")))?;
            client.cache().put_resolve(kind, ref_, &id);
            Ok(id)
        }
        _ => Err(Error::Api(format!(
            "{kind} {ref_:?} is ambiguous. Pass a numeric id."
        ))),
    }
}

/// Resolve a status reference to its id.
pub async fn status_id(client: &Client, ref_: &str) -> Result<String, Error> {
    resolve_by_name(client, "statuses", ref_, "Status").await
}

/// Resolve a work-package type reference to its id.
pub async fn type_id(client: &Client, ref_: &str) -> Result<String, Error> {
    resolve_by_name(client, "types", ref_, "Type").await
}

/// Resolve a time-entry activity reference to its id.
pub async fn activity_id(client: &Client, ref_: &str) -> Result<String, Error> {
    resolve_by_name(client, "time_entries/activities", ref_, "Activity").await
}

/// Resolve a principal (user/group) reference to its numeric id. Accepts the
/// literal `me`, a numeric id (passthrough), or a name matched by tokens.
pub async fn resolve_principal_id(client: &Client, ref_: &str) -> Result<String, Error> {
    let ref_ = ref_.trim();
    if ref_.to_lowercase() == "me" {
        let me = client.current_user().await?;
        return match me.get("id") {
            Some(serde_json::Value::Number(n)) => Ok(n.to_string()),
            Some(serde_json::Value::String(s)) => Ok(s.clone()),
            _ => Err(Error::Api(
                "Principal 'me' was not found. Pass a numeric user id or 'me'.".to_string(),
            )),
        };
    }
    if is_ascii_digits(ref_) {
        return Ok(ref_.to_string());
    }
    let lower = ref_.to_lowercase();
    let tokens: Vec<&str> = lower.split_whitespace().collect();
    if tokens.is_empty() {
        return Err(Error::Api(
            "Principal '' was not found. Pass a numeric user id or 'me'.".to_string(),
        ));
    }
    let probe = tokens.iter().max_by_key(|t| t.len()).copied().unwrap_or("");
    let filters = json!([{"name": {"operator": "~", "values": [probe]}}]);
    let filters_json = serde_json::to_string(&filters)
        .map_err(|e| Error::Internal(format!("encode filters: {e}")))?;
    let elements = client
        .collect("principals", &[("filters".to_string(), filters_json)])
        .await?;

    let exact: Vec<&serde_json::Value> = elements
        .iter()
        .filter(|e| {
            e.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n.to_lowercase() == lower)
                .unwrap_or(false)
        })
        .collect();
    let candidates: Vec<&serde_json::Value> = if !exact.is_empty() {
        exact
    } else {
        elements
            .iter()
            .filter(|e| {
                e.get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| {
                        let nl = n.to_lowercase();
                        tokens.iter().all(|t| nl.contains(t))
                    })
                    .unwrap_or(false)
            })
            .collect()
    };

    match candidates.len() {
        1 => id_to_string(candidates[0])
            .ok_or_else(|| Error::Api(format!("Principal {ref_:?} has no id"))),
        0 => Err(Error::Api(format!(
            "Principal {ref_:?} was not found. Pass a numeric user id or 'me'."
        ))),
        n => {
            let list = candidates
                .iter()
                .map(|e| {
                    let id = id_to_string(e).unwrap_or_default();
                    let name = e.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    format!("{id}: {name}")
                })
                .collect::<Vec<_>>()
                .join("; ");
            Err(Error::Api(format!(
                "Principal {ref_:?} is ambiguous ({n} matches): {list}. Pass a numeric user id."
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerProfile;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn profile_for(url: &str) -> ServerProfile {
        ServerProfile {
            base_url: url.into(),
            timeout: 30,
            verify_ssl: true,
            proxy: None,
        }
    }

    fn client_for(server: &MockServer, name: &str) -> Client {
        Client::new(name, &profile_for(&server.uri()), "t".into(), Some("none")).unwrap()
    }

    #[tokio::test]
    async fn principal_me_reads_current_user() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/users/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": 3})))
            .mount(&server)
            .await;
        let c = client_for(&server, "res-me");
        assert_eq!(resolve_principal_id(&c, "me").await.unwrap(), "3");
    }

    #[tokio::test]
    async fn principal_numeric_passthrough() {
        let server = MockServer::start().await;
        let c = client_for(&server, "res-num");
        assert_eq!(resolve_principal_id(&c, "42").await.unwrap(), "42");
    }

    #[tokio::test]
    async fn principal_single_exact_match() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/principals"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total": 2,
                "_embedded": {"elements": [
                    {"id": 5, "name": "Ivan Petrov"},
                    {"id": 6, "name": "Ivan Petrovich"}
                ]}
            })))
            .mount(&server)
            .await;
        let c = client_for(&server, "res-exact");
        // "Ivan Petrov" is an exact match despite the substring sibling.
        assert_eq!(resolve_principal_id(&c, "Ivan Petrov").await.unwrap(), "5");
    }

    #[tokio::test]
    async fn principal_ambiguous_reports_matches() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/principals"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total": 2,
                "_embedded": {"elements": [
                    {"id": 5, "name": "Ivan Petrov Sr"},
                    {"id": 6, "name": "Petrov Ivan Jr"}
                ]}
            })))
            .mount(&server)
            .await;
        let c = client_for(&server, "res-amb");
        let err = resolve_principal_id(&c, "ivan petrov")
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("is ambiguous"), "got: {err}");
        assert!(err.contains("5: Ivan Petrov Sr"), "got: {err}");
    }

    #[tokio::test]
    async fn principal_not_found() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/principals"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total": 0,
                "_embedded": {"elements": []}
            })))
            .mount(&server)
            .await;
        let c = client_for(&server, "res-nf");
        let err = resolve_principal_id(&c, "nobody here")
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("was not found"), "got: {err}");
    }

    #[tokio::test]
    async fn status_numeric_passthrough() {
        let server = MockServer::start().await;
        let c = client_for(&server, "st-num");
        assert_eq!(status_id(&c, "12").await.unwrap(), "12");
    }

    #[tokio::test]
    async fn status_name_match() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/statuses"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total": 2,
                "_embedded": {"elements": [
                    {"id": 1, "name": "New"},
                    {"id": 2, "name": "Closed"}
                ]}
            })))
            .mount(&server)
            .await;
        let c = client_for(&server, "st-name");
        assert_eq!(status_id(&c, "closed").await.unwrap(), "2");
    }

    #[tokio::test]
    async fn status_not_found_lists_available() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/statuses"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total": 2,
                "_embedded": {"elements": [
                    {"id": 2, "name": "Closed"},
                    {"id": 1, "name": "New"}
                ]}
            })))
            .mount(&server)
            .await;
        let c = client_for(&server, "st-nf");
        let err = status_id(&c, "foo").await.unwrap_err().to_string();
        assert_eq!(
            err,
            "api: Status \"foo\" was not found. Available: Closed, New"
        );
    }

    #[tokio::test]
    async fn status_ambiguous() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/statuses"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total": 2,
                "_embedded": {"elements": [
                    {"id": 1, "name": "New"},
                    {"id": 3, "name": "new"}
                ]}
            })))
            .mount(&server)
            .await;
        let c = client_for(&server, "st-amb");
        let err = status_id(&c, "NEW").await.unwrap_err().to_string();
        assert!(
            err.contains("is ambiguous. Pass a numeric id."),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn status_name_is_cached_no_second_http() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("OPENPROJECT_CACHE", tmp.path());
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/statuses"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total": 1,
                "_embedded": {"elements": [{"id": 1, "name": "New"}]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "st-cache");
        assert_eq!(status_id(&c, "New").await.unwrap(), "1");
        // Served from cache; the mock asserts exactly one HTTP call on drop.
        assert_eq!(status_id(&c, "New").await.unwrap(), "1");
    }

    #[tokio::test]
    async fn principal_probe_uses_longest_token() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/principals"))
            .and(query_param(
                "filters",
                "[{\"name\":{\"operator\":\"~\",\"values\":[\"petrov\"]}}]",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total": 1,
                "_embedded": {"elements": [{"id": 9, "name": "Ivan Petrov"}]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "res-probe");
        assert_eq!(resolve_principal_id(&c, "ivan petrov").await.unwrap(), "9");
    }
}
