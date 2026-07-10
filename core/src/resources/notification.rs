//! Notification resource operations (`notification` command family): list, read,
//! unread. Read-only plus the read/unread state toggles. Ported 1:1 from the
//! predecessor Python `notification.py`.

use serde_json::{json, Value};

use crate::client::Client;
use crate::error::Error;
use crate::normalize;

/// Pagination query pairs: always `offset`, plus `pageSize` when a limit is set.
fn paging_query(offset: i64, limit: Option<i64>) -> Vec<(String, String)> {
    let mut q = vec![("offset".to_string(), offset.to_string())];
    if let Some(l) = limit {
        q.push(("pageSize".to_string(), l.to_string()));
    }
    q
}

/// Fetch one page of notifications (newest first) and the reported total.
/// `offset` is the 1-based page number.
pub async fn list_page(
    client: &Client,
    offset: i64,
    limit: Option<i64>,
    raw: bool,
) -> Result<(Value, i64), Error> {
    let mut q = paging_query(offset, limit);
    q.push(("sortBy".to_string(), json!([["id", "desc"]]).to_string()));
    let payload = client
        .request_json_query(reqwest::Method::GET, "notifications", &q, None)
        .await?;
    let total = payload.get("total").and_then(|t| t.as_i64()).unwrap_or(0);
    let elements = normalize::collection(&payload);
    if raw {
        return Ok((Value::Array(elements), total));
    }
    let out: Vec<Value> = elements.iter().map(normalize::notification).collect();
    Ok((Value::Array(out), total))
}

/// List notifications, newest first. Returns normalized notifications unless
/// `raw` is set, in which case the raw collection elements are returned.
pub async fn list(
    client: &Client,
    offset: i64,
    limit: Option<i64>,
    raw: bool,
) -> Result<Value, Error> {
    let (page, _total) = list_page(client, offset, limit, raw).await?;
    Ok(page)
}

/// Mark a notification as read.
pub async fn read(client: &Client, id: i64) -> Result<Value, Error> {
    client
        .post_empty_json(&format!("notifications/{id}/read_ian"))
        .await?;
    Ok(json!({ "read": id }))
}

/// Mark a notification as unread.
pub async fn unread(client: &Client, id: i64) -> Result<Value, Error> {
    client
        .post_empty_json(&format!("notifications/{id}/unread_ian"))
        .await?;
    Ok(json!({ "unread": id }))
}

/// Mark every currently-unread notification as read. Returns the count marked.
pub async fn read_all(client: &Client) -> Result<Value, Error> {
    let list = list(client, 1, None, false).await?;
    let mut count = 0;
    if let Value::Array(items) = &list {
        for n in items {
            let unread = n.get("read").and_then(|v| v.as_bool()) != Some(true);
            if unread {
                if let Some(id) = n.get("id").and_then(|v| v.as_i64()) {
                    read(client, id).await?;
                    count += 1;
                }
            }
        }
    }
    Ok(json!({ "read": count }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerProfile;
    use wiremock::matchers::{method, path, query_param};
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
        }
    }

    fn client_for(server: &MockServer, name: &str) -> Client {
        Client::new(name, &profile_for(&server.uri()), "t".into(), Some("none")).unwrap()
    }

    #[tokio::test]
    async fn list_sorts_by_id_desc_and_normalizes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/notifications"))
            .and(query_param("sortBy", r#"[["id","desc"]]"#))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_embedded": {"elements": [{
                    "id": 3,
                    "reason": "mentioned",
                    "readIAN": false,
                    "_links": {
                        "resource": {"href": "/api/v3/work_packages/9", "title": "WP"},
                        "activity": {"href": "/api/v3/activities/12"}
                    },
                    "createdAt": "2026-01-01T00:00:00Z"
                }]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "notification-list");
        let out = list(&c, 1, None, false).await.unwrap();
        let arr = out.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], json!(3));
        assert_eq!(arr[0]["read"], json!(false));
        assert_eq!(arr[0]["wpId"], json!(9));
    }

    #[tokio::test]
    async fn list_page_returns_reported_total() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/notifications"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total": 7,
                "_embedded": {"elements": [
                    {"id": 3, "reason": "mentioned", "readIAN": false}
                ]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "notification-list-page");
        let (page, total) = list_page(&c, 1, Some(50), false).await.unwrap();
        assert_eq!(total, 7);
        assert_eq!(page.as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn read_posts_read_ian_and_returns_id() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v3/notifications/7/read_ian"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "notification-read");
        let out = read(&c, 7).await.unwrap();
        assert_eq!(out, json!({"read": 7}));
    }

    #[tokio::test]
    async fn read_all_marks_only_unread() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/notifications"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_embedded": {"elements": [
                    {"id": 1, "reason": "mentioned", "readIAN": false},
                    {"id": 2, "reason": "assigned", "readIAN": true},
                    {"id": 3, "reason": "watched", "readIAN": false}
                ]}
            })))
            .mount(&server)
            .await;
        // Only the two unread ids are read; the already-read one is not.
        Mock::given(method("POST"))
            .and(path("/api/v3/notifications/1/read_ian"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v3/notifications/3/read_ian"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "notification-read-all");
        let out = read_all(&c).await.unwrap();
        assert_eq!(out, json!({"read": 2}));
    }

    #[tokio::test]
    async fn unread_posts_unread_ian_and_returns_id() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v3/notifications/7/unread_ian"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "notification-unread");
        let out = unread(&c, 7).await.unwrap();
        assert_eq!(out, json!({"unread": 7}));
    }
}
