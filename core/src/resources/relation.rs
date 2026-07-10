//! Relation resource operations (`relation` command family): list, get, create,
//! update, delete. Ported 1:1 from the predecessor Python `relation.py`.
//!
//! Note the two special cases mirrored from the original tool: a relation's
//! `description` is a plain string (not a `{raw}` formattable object), and
//! `update` only carries `lockVersion` when the current relation reports one.

use serde_json::{json, Value};

use crate::client::Client;
use crate::error::Error;
use crate::normalize;

/// Accepted relation type values. Value validation of `--type` is performed by
/// the CLI (via clap) against this list; it is exported from here.
pub const RELATION_TYPES: &[&str] = &[
    "relates",
    "duplicates",
    "duplicated",
    "blocks",
    "blocked",
    "precedes",
    "follows",
    "includes",
    "partof",
    "requires",
    "required",
];

/// Pagination query pairs: always `offset`, plus `pageSize` when a limit is set.
fn paging_query(offset: i64, limit: Option<i64>) -> Vec<(String, String)> {
    let mut q = vec![("offset".to_string(), offset.to_string())];
    if let Some(l) = limit {
        q.push(("pageSize".to_string(), l.to_string()));
    }
    q
}

/// List the relations of a work package, optionally filtered by relation type.
pub async fn list(
    client: &Client,
    work_package: i64,
    type_: Option<&str>,
    offset: i64,
    limit: Option<i64>,
    raw: bool,
) -> Result<Value, Error> {
    let mut q = paging_query(offset, limit);
    if let Some(t) = type_ {
        let filters = json!([{"type": {"operator": "=", "values": [t]}}]);
        q.push(("filters".to_string(), filters.to_string()));
    }
    let payload = client
        .request_json_query(
            reqwest::Method::GET,
            &format!("work_packages/{work_package}/relations"),
            &q,
            None,
        )
        .await?;
    let elements = normalize::collection(&payload);
    if raw {
        return Ok(Value::Array(elements));
    }
    let out: Vec<Value> = elements.iter().map(normalize::relation).collect();
    Ok(Value::Array(out))
}

/// Fetch a single relation by id.
pub async fn get(client: &Client, id: i64, raw: bool) -> Result<Value, Error> {
    let payload = client
        .request_json(reqwest::Method::GET, &format!("relations/{id}"), None)
        .await?;
    if raw {
        Ok(payload)
    } else {
        Ok(normalize::relation(&payload))
    }
}

/// Create a relation from a work package to another. The relation `description`
/// is sent as a bare string when present.
pub async fn create(
    client: &Client,
    work_package: i64,
    to: i64,
    type_: &str,
    description: Option<&str>,
    raw: bool,
) -> Result<Value, Error> {
    let mut body = json!({
        "type": type_,
        "_links": {"to": {"href": format!("/api/v3/work_packages/{to}")}}
    });
    if let Some(desc) = description {
        body["description"] = Value::String(desc.to_owned());
    }
    let payload = client
        .post_json(&format!("work_packages/{work_package}/relations"), body)
        .await?;
    if raw {
        Ok(payload)
    } else {
        Ok(normalize::relation(&payload))
    }
}

/// Update a relation, carrying its current `lockVersion` when present. The
/// `type` and `description` fields are sent as bare strings.
pub async fn update(
    client: &Client,
    id: i64,
    type_: Option<&str>,
    description: Option<&str>,
    raw: bool,
) -> Result<Value, Error> {
    let current = client
        .request_json(reqwest::Method::GET, &format!("relations/{id}"), None)
        .await?;
    let lock_version = current.get("lockVersion").cloned().unwrap_or(Value::Null);
    let mut body = json!({});
    if let Some(map) = body.as_object_mut() {
        if lock_version != Value::Null {
            map.insert("lockVersion".to_string(), lock_version);
        }
        if let Some(t) = type_ {
            map.insert("type".to_string(), Value::String(t.to_owned()));
        }
        if let Some(desc) = description {
            map.insert("description".to_string(), Value::String(desc.to_owned()));
        }
    }
    let payload = client.patch_json(&format!("relations/{id}"), body).await?;
    if raw {
        Ok(payload)
    } else {
        Ok(normalize::relation(&payload))
    }
}

/// Delete a relation by id. Returns `{"deleted": id}` on success.
pub async fn delete(client: &Client, id: i64) -> Result<Value, Error> {
    client.delete(&format!("relations/{id}")).await?;
    Ok(json!({"deleted": id}))
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

    fn relation_element(id: i64, type_: &str) -> Value {
        json!({
            "id": id,
            "type": type_,
            "reverseType": type_,
            "description": "d",
            "lockVersion": 0,
            "_links": {
                "from": {"href": "/api/v3/work_packages/1"},
                "to": {"href": "/api/v3/work_packages/2"}
            }
        })
    }

    #[tokio::test]
    async fn list_with_type_sends_filter_and_normalizes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/work_packages/1/relations"))
            .and(query_param(
                "filters",
                r#"[{"type":{"operator":"=","values":["relates"]}}]"#,
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_embedded": {"elements": [relation_element(6, "relates")]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "relation-list");
        let out = list(&c, 1, Some("relates"), 1, None, false).await.unwrap();
        let arr = out.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], json!(6));
        assert_eq!(arr[0]["from"], json!(1));
        assert_eq!(arr[0]["to"], json!(2));
    }

    #[tokio::test]
    async fn get_normalizes_relation() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/relations/6"))
            .respond_with(ResponseTemplate::new(200).set_body_json(relation_element(6, "blocks")))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "relation-get");
        let out = get(&c, 6, false).await.unwrap();
        assert_eq!(out["id"], json!(6));
        assert_eq!(out["type"], json!("blocks"));
        assert_eq!(out["to"], json!(2));
    }

    #[tokio::test]
    async fn create_sends_type_link_and_string_description() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v3/work_packages/1/relations"))
            .respond_with(ResponseTemplate::new(201).set_body_json(relation_element(7, "relates")))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "relation-create");
        let out = create(&c, 1, 2, "relates", Some("because"), false)
            .await
            .unwrap();
        assert_eq!(out["id"], json!(7));

        let requests = server.received_requests().await.unwrap();
        let post = requests
            .iter()
            .find(|r| r.method == wiremock::http::Method::POST)
            .unwrap();
        let body: Value = serde_json::from_slice(&post.body).unwrap();
        assert_eq!(body["type"], json!("relates"));
        assert_eq!(
            body["_links"]["to"]["href"],
            json!("/api/v3/work_packages/2")
        );
        assert_eq!(body["description"], json!("because"));
        assert!(body["description"].is_string());
    }

    #[tokio::test]
    async fn update_reads_lock_version_and_sends_strings() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/relations/3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 3, "lockVersion": 5
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("PATCH"))
            .and(path("/api/v3/relations/3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(relation_element(3, "blocks")))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "relation-update");
        let out = update(&c, 3, Some("blocks"), Some("note"), false)
            .await
            .unwrap();
        assert_eq!(out["id"], json!(3));

        let requests = server.received_requests().await.unwrap();
        let patch = requests
            .iter()
            .find(|r| r.method == wiremock::http::Method::PATCH)
            .unwrap();
        let body: Value = serde_json::from_slice(&patch.body).unwrap();
        assert_eq!(body["lockVersion"], json!(5));
        assert_eq!(body["type"], json!("blocks"));
        assert!(body["type"].is_string());
        assert_eq!(body["description"], json!("note"));
        assert!(body["description"].is_string());
    }

    #[tokio::test]
    async fn update_omits_lock_version_when_absent() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/relations/4"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": 4})))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("PATCH"))
            .and(path("/api/v3/relations/4"))
            .respond_with(ResponseTemplate::new(200).set_body_json(relation_element(4, "relates")))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "relation-update-nolock");
        update(&c, 4, Some("relates"), None, true).await.unwrap();

        let requests = server.received_requests().await.unwrap();
        let patch = requests
            .iter()
            .find(|r| r.method == wiremock::http::Method::PATCH)
            .unwrap();
        let body: Value = serde_json::from_slice(&patch.body).unwrap();
        assert!(body.get("lockVersion").is_none());
        assert_eq!(body["type"], json!("relates"));
    }

    #[tokio::test]
    async fn delete_reports_deleted_id() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v3/relations/9"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "relation-delete");
        let out = delete(&c, 9).await.unwrap();
        assert_eq!(out, json!({"deleted": 9}));

        let requests = server.received_requests().await.unwrap();
        assert!(requests
            .iter()
            .any(|r| r.method == wiremock::http::Method::DELETE));
    }
}
