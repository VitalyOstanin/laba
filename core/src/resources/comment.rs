//! Comment (activity) resource operations (`comment` command family): list, get,
//! create, update. There is no delete. Ported 1:1 from the predecessor Python
//! `comment.py`.

use futures_util::future::try_join_all;
use serde_json::{json, Value};

use crate::client::Client;
use crate::error::Error;
use crate::normalize;

/// Pagination query pairs: always `offset`, plus `pageSize` when a limit is set.
/// The activities endpoint is not paginated (the server ignores these), but the
/// parameters are still sent to match the original tool.
fn paging_query(offset: i64, limit: Option<i64>) -> Vec<(String, String)> {
    let mut q = vec![("offset".to_string(), offset.to_string())];
    if let Some(l) = limit {
        q.push(("pageSize".to_string(), l.to_string()));
    }
    q
}

/// Normalize a comment and resolve its author name when only a user id is known:
/// if the normalized `user` is null but `userId` is present, look the name up.
async fn resolved_comment(client: &Client, item: &Value) -> Result<Value, Error> {
    let mut n = normalize::comment(item);
    if n.get("user") == Some(&Value::Null) {
        if let Some(uid) = n.get("userId").and_then(|v| v.as_i64()) {
            if let Some(name) = client.user_name(uid).await? {
                n["user"] = Value::String(name);
            }
        }
    }
    Ok(n)
}

/// Whether a raw activity element carries a non-empty comment body.
fn has_comment_text(item: &Value) -> bool {
    item.get("comment")
        .and_then(|c| c.get("raw"))
        .and_then(|r| r.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

/// List the activities of a work package. With `comments_only`, drop elements
/// that carry no comment body.
pub async fn list(
    client: &Client,
    work_package: i64,
    comments_only: bool,
    offset: i64,
    limit: Option<i64>,
    raw: bool,
) -> Result<Value, Error> {
    let q = paging_query(offset, limit);
    let payload = client
        .request_json_query(
            reqwest::Method::GET,
            &format!("work_packages/{work_package}/activities"),
            &q,
            None,
        )
        .await?;
    let mut elements: Vec<Value> = payload
        .get("_embedded")
        .and_then(|e| e.get("elements"))
        .and_then(|e| e.as_array())
        .cloned()
        .unwrap_or_default();
    if comments_only {
        elements.retain(has_comment_text);
    }
    if raw {
        return Ok(Value::Array(elements));
    }
    // Resolve author names concurrently: each unique author whose name is not
    // embedded needs a `users/{id}` lookup, and a comment thread can hold many
    // distinct authors. Sequential `await`s would be a round-trip per author;
    // `try_join_all` overlaps them (name lookups are cached, so repeats are free).
    // Order is preserved, matching the source element order.
    let out = try_join_all(elements.iter().map(|e| resolved_comment(client, e))).await?;
    Ok(Value::Array(out))
}

/// Fetch a single activity by id.
pub async fn get(client: &Client, id: i64, raw: bool) -> Result<Value, Error> {
    let payload = client
        .request_json(reqwest::Method::GET, &format!("activities/{id}"), None)
        .await?;
    if raw {
        Ok(payload)
    } else {
        resolved_comment(client, &payload).await
    }
}

/// Create a comment on a work package. The API expects the comment body wrapped
/// as a formattable object: `{"comment": {"raw": text}}`.
pub async fn create(
    client: &Client,
    work_package: i64,
    text: &str,
    raw: bool,
) -> Result<Value, Error> {
    let body = json!({"comment": {"raw": text}});
    let payload = client
        .post_json(&format!("work_packages/{work_package}/activities"), body)
        .await?;
    if raw {
        Ok(payload)
    } else {
        Ok(normalize::comment(&payload))
    }
}

/// Update an existing comment, carrying its current `lockVersion`. The update
/// endpoint expects the comment body as a bare string: `{"comment": text}`.
pub async fn update(client: &Client, id: i64, text: &str, raw: bool) -> Result<Value, Error> {
    let current = client
        .request_json(reqwest::Method::GET, &format!("activities/{id}"), None)
        .await?;
    let lock_version = current.get("lockVersion").cloned().unwrap_or(Value::Null);
    let mut body = json!({"comment": text});
    if lock_version != Value::Null {
        if let Some(map) = body.as_object_mut() {
            map.insert("lockVersion".to_string(), lock_version);
        }
    }
    let payload = client.patch_json(&format!("activities/{id}"), body).await?;
    if raw {
        Ok(payload)
    } else {
        Ok(normalize::comment(&payload))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerProfile;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn profile_for(url: &str) -> ServerProfile {
        ServerProfile {
            backend: Default::default(),
            base_url: url.into(),
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
        }
    }

    fn client_for(server: &MockServer, name: &str) -> Client {
        Client::new(name, &profile_for(&server.uri()), "t".into(), Some("none")).unwrap()
    }

    fn comment_element(id: i64, text: &str) -> Value {
        json!({
            "id": id,
            "_type": "Activity::Comment",
            "comment": {"raw": text},
            "createdAt": "2026-01-01T00:00:00Z",
            "version": 1,
            "_links": {"user": {"href": "/api/v3/users/8", "title": "U"}}
        })
    }

    #[tokio::test]
    async fn list_normalizes_comments() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/work_packages/5/activities"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_embedded": {"elements": [comment_element(1, "hi")]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "comment-list");
        let out = list(&c, 5, false, 1, None, false).await.unwrap();
        let arr = out.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], json!(1));
        assert_eq!(arr[0]["comment"], json!("hi"));
        assert_eq!(arr[0]["user"], json!("U"));
    }

    #[tokio::test]
    async fn list_comments_only_filters_empty() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/work_packages/5/activities"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_embedded": {"elements": [
                    comment_element(1, "hi"),
                    json!({"id": 2, "_type": "Activity", "comment": {"raw": ""}}),
                    json!({"id": 3, "_type": "Activity"})
                ]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "comment-list-only");
        let out = list(&c, 5, true, 1, None, true).await.unwrap();
        let arr = out.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], json!(1));
    }

    #[tokio::test]
    async fn resolved_comment_looks_up_user_name() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("OPENPROJECT_CACHE", tmp.path());
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/work_packages/5/activities"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_embedded": {"elements": [json!({
                    "id": 1,
                    "_type": "Activity::Comment",
                    "comment": {"raw": "hi"},
                    "_links": {"user": {"href": "/api/v3/users/9"}}
                })]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v3/users/9"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"name": "Ivan"})))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "comment-resolve-user");
        let out = list(&c, 5, false, 1, None, false).await.unwrap();
        assert_eq!(out.as_array().unwrap()[0]["user"], json!("Ivan"));
    }

    #[tokio::test]
    async fn list_resolves_multiple_authors_in_order() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("OPENPROJECT_CACHE", tmp.path());
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/work_packages/5/activities"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_embedded": {"elements": [
                    json!({
                        "id": 1,
                        "_type": "Activity::Comment",
                        "comment": {"raw": "first"},
                        "_links": {"user": {"href": "/api/v3/users/9"}}
                    }),
                    json!({
                        "id": 2,
                        "_type": "Activity::Comment",
                        "comment": {"raw": "second"},
                        "_links": {"user": {"href": "/api/v3/users/10"}}
                    })
                ]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v3/users/9"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"name": "Ivan"})))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v3/users/10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"name": "Olga"})))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "comment-resolve-multi");
        let out = list(&c, 5, false, 1, None, false).await.unwrap();
        let arr = out.as_array().unwrap();
        // Both authors resolved and the source order is preserved.
        assert_eq!(arr[0]["id"], json!(1));
        assert_eq!(arr[0]["user"], json!("Ivan"));
        assert_eq!(arr[1]["id"], json!(2));
        assert_eq!(arr[1]["user"], json!("Olga"));
    }

    #[tokio::test]
    async fn create_sends_raw_object_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v3/work_packages/5/activities"))
            .respond_with(ResponseTemplate::new(201).set_body_json(comment_element(7, "new")))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "comment-create");
        let out = create(&c, 5, "new", false).await.unwrap();
        assert_eq!(out["id"], json!(7));

        let requests = server.received_requests().await.unwrap();
        let post = requests
            .iter()
            .find(|r| r.method == wiremock::http::Method::POST)
            .unwrap();
        let body: Value = serde_json::from_slice(&post.body).unwrap();
        assert_eq!(body, json!({"comment": {"raw": "new"}}));
        assert!(body["comment"].is_object());
    }

    #[tokio::test]
    async fn update_reads_lock_version_and_sends_string_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/activities/3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 3, "lockVersion": 2
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("PATCH"))
            .and(path("/api/v3/activities/3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(comment_element(3, "edited")))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "comment-update");
        let out = update(&c, 3, "edited", false).await.unwrap();
        assert_eq!(out["id"], json!(3));

        let requests = server.received_requests().await.unwrap();
        let patch = requests
            .iter()
            .find(|r| r.method == wiremock::http::Method::PATCH)
            .unwrap();
        let body: Value = serde_json::from_slice(&patch.body).unwrap();
        assert_eq!(body["comment"], json!("edited"));
        assert!(body["comment"].is_string());
        assert_eq!(body["lockVersion"], json!(2));
    }
}
