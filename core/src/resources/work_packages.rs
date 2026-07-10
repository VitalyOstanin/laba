//! Work package resource operations (`wp` command family): list, query, search,
//! get, create, update, delete. Ported 1:1 from the predecessor Python
//! `work_packages.py`.

use std::collections::BTreeSet;

use serde_json::{json, Map, Value};

use crate::client::Client;
use crate::error::Error;
use crate::{normalize, resolve, state};

/// Parameters for [`list`].
#[derive(Debug, Default, Clone)]
pub struct WpListParams {
    pub project: Option<String>,
    pub status: Option<String>,
    pub type_: Option<String>,
    pub assignee: Option<String>,
    pub subject: Option<String>,
    pub open: bool,
    pub include_past: bool,
    pub offset: i64,
    pub limit: Option<i64>,
}

/// Mutable fields for [`create`] and [`update`].
#[derive(Debug, Default, Clone)]
pub struct WpFields {
    pub subject: Option<String>,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub done_ratio: Option<i64>,
    pub project: Option<String>,
    pub type_: Option<String>,
    pub status: Option<String>,
    pub assignee: Option<String>,
    pub parent: Option<String>,
}

/// Pagination query pairs: always `offset`, plus `pageSize` when a limit is set.
fn paging_query(offset: i64, limit: Option<i64>) -> Vec<(String, String)> {
    let mut q = vec![("offset".to_string(), offset.to_string())];
    if let Some(l) = limit {
        q.push(("pageSize".to_string(), l.to_string()));
    }
    q
}

/// Build an API href of the form `/api/v3/{kind}/{ref}`.
fn href(kind: &str, ref_: &str) -> String {
    format!("/api/v3/{kind}/{ref_}")
}

/// Serialize a filter array to the compact JSON string the API expects.
fn filters_json(filters: &[Value]) -> Result<String, Error> {
    serde_json::to_string(&Value::Array(filters.to_vec()))
        .map_err(|e| Error::Internal(format!("encode filters: {e}")))
}

/// Assemble the filter array in the fixed order used by the Python tool.
fn make_filters(
    project_id: &Option<String>,
    status_id: &Option<String>,
    open: bool,
    type_id: &Option<String>,
    assignee_id: &Option<String>,
    subject: &Option<String>,
) -> Vec<Value> {
    let mut f = Vec::new();
    if let Some(p) = project_id {
        f.push(json!({"project_id": {"operator": "=", "values": [p]}}));
    }
    if let Some(s) = status_id {
        f.push(json!({"status_id": {"operator": "=", "values": [s]}}));
    }
    if open {
        f.push(json!({"status_id": {"operator": "o", "values": []}}));
    }
    if let Some(t) = type_id {
        f.push(json!({"type": {"operator": "=", "values": [t]}}));
    }
    if let Some(a) = assignee_id {
        f.push(json!({"assignee": {"operator": "=", "values": [a]}}));
    }
    if let Some(s) = subject {
        f.push(json!({"subject": {"operator": "~", "values": [s]}}));
    }
    f
}

/// Normalize a work package and append its expanded custom fields under the
/// `customFields` key (inserted last, preserving column order).
async fn with_custom_fields(client: &Client, item: &Value) -> Result<Value, Error> {
    let mut norm = normalize::work_package(item);
    let cf = client.custom_fields(item).await?;
    if let Some(map) = norm.as_object_mut() {
        map.insert("customFields".to_string(), Value::Array(cf));
    }
    Ok(norm)
}

/// Render a list of raw elements: as-is when `raw`, else normalized with custom
/// fields.
async fn render_elements(client: &Client, elements: Vec<Value>, raw: bool) -> Result<Value, Error> {
    if raw {
        return Ok(Value::Array(elements));
    }
    let mut out = Vec::with_capacity(elements.len());
    for e in &elements {
        out.push(with_custom_fields(client, e).await?);
    }
    Ok(Value::Array(out))
}

/// Embedded `_embedded.elements` array of a HAL payload, or empty.
fn embedded_elements(payload: &Value) -> Vec<Value> {
    payload
        .get("_embedded")
        .and_then(|e| e.get("elements"))
        .and_then(|e| e.as_array())
        .cloned()
        .unwrap_or_default()
}

/// Build the request body (scalars + `_links`) for create/update.
async fn build_links_and_body(
    client: &Client,
    for_create: bool,
    fields: &WpFields,
) -> Result<Value, Error> {
    let mut body: Map<String, Value> = Map::new();
    if let Some(s) = &fields.subject {
        body.insert("subject".to_string(), json!(s));
    }
    if let Some(d) = &fields.description {
        body.insert("description".to_string(), json!({"raw": d}));
    }
    if let Some(sd) = &fields.start_date {
        body.insert("startDate".to_string(), json!(sd));
    }
    if let Some(dd) = &fields.due_date {
        body.insert("dueDate".to_string(), json!(dd));
    }
    if let Some(pd) = fields.done_ratio {
        body.insert("percentageDone".to_string(), json!(pd));
    }

    let mut links: Map<String, Value> = Map::new();
    if for_create {
        let project_ref = fields.project.clone().unwrap_or_default();
        let pid = resolve::project_id(client, &project_ref).await?;
        links.insert(
            "project".to_string(),
            json!({"href": href("projects", &pid)}),
        );
        let type_ref = fields.type_.clone().unwrap_or_default();
        let tid = resolve::type_id(client, &type_ref).await?;
        links.insert("type".to_string(), json!({"href": href("types", &tid)}));
    }
    if let Some(s) = &fields.status {
        let id = resolve::status_id(client, s).await?;
        links.insert("status".to_string(), json!({"href": href("statuses", &id)}));
    }
    if !for_create {
        if let Some(t) = &fields.type_ {
            let id = resolve::type_id(client, t).await?;
            links.insert("type".to_string(), json!({"href": href("types", &id)}));
        }
    }
    if let Some(a) = &fields.assignee {
        let id = resolve::resolve_principal_id(client, a).await?;
        links.insert("assignee".to_string(), json!({"href": href("users", &id)}));
    }
    if let Some(p) = &fields.parent {
        if !p.is_empty() {
            links.insert(
                "parent".to_string(),
                json!({"href": href("work_packages", p)}),
            );
        }
    }

    if !links.is_empty() {
        body.insert("_links".to_string(), Value::Object(links));
    }
    Ok(Value::Object(body))
}

/// Fetch one page of the filtered work-package list and the reported total.
/// Only the paged (non-`include_past`) path; the `include_past` aggregation in
/// [`list`] is not paginated. `params.offset` is the 1-based page number.
pub async fn list_page(
    client: &Client,
    params: WpListParams,
    raw: bool,
) -> Result<(Value, i64), Error> {
    let project_id = match &params.project {
        Some(p) => Some(resolve::project_id(client, p).await?),
        None => None,
    };
    let status_id = match &params.status {
        Some(s) => Some(resolve::status_id(client, s).await?),
        None => None,
    };
    let type_id = match &params.type_ {
        Some(t) => Some(resolve::type_id(client, t).await?),
        None => None,
    };
    let assignee_id = match &params.assignee {
        Some(a) => Some(resolve::resolve_principal_id(client, a).await?),
        None => None,
    };
    let filters = make_filters(
        &project_id,
        &status_id,
        params.open,
        &type_id,
        &assignee_id,
        &params.subject,
    );
    let mut q = paging_query(params.offset, params.limit);
    if !filters.is_empty() {
        q.push(("filters".to_string(), filters_json(&filters)?));
    }
    let payload = client
        .request_json_query(reqwest::Method::GET, "work_packages", &q, None)
        .await?;
    let total = payload.get("total").and_then(|t| t.as_i64()).unwrap_or(0);
    let elements = embedded_elements(&payload);
    let rendered = render_elements(client, elements, raw).await?;
    Ok((rendered, total))
}

/// List work packages by filters. With `include_past`, also merges in work
/// packages the user was previously assigned to (requires `--assignee`).
pub async fn list(client: &Client, params: WpListParams, raw: bool) -> Result<Value, Error> {
    if !params.include_past {
        let (page, _total) = list_page(client, params, raw).await?;
        return Ok(page);
    }

    let project_id = match &params.project {
        Some(p) => Some(resolve::project_id(client, p).await?),
        None => None,
    };
    let status_id = match &params.status {
        Some(s) => Some(resolve::status_id(client, s).await?),
        None => None,
    };
    let type_id = match &params.type_ {
        Some(t) => Some(resolve::type_id(client, t).await?),
        None => None,
    };

    // include_past path.
    let assignee = params
        .assignee
        .as_ref()
        .ok_or_else(|| Error::Usage("--include-past requires --assignee".to_string()))?;
    let uid = resolve::resolve_principal_id(client, assignee).await?;

    let all_filters = make_filters(
        &project_id,
        &status_id,
        params.open,
        &type_id,
        &Some(uid.clone()),
        &params.subject,
    );
    let current_query: Vec<(String, String)> = if all_filters.is_empty() {
        Vec::new()
    } else {
        vec![("filters".to_string(), filters_json(&all_filters)?)]
    };
    let current = client.collect("work_packages", &current_query).await?;

    let current_ids: BTreeSet<i64> = current
        .iter()
        .filter_map(|e| e.get("id").and_then(|v| v.as_i64()))
        .collect();

    let history = {
        let (base, uid) = (client.base_url().to_owned(), uid.clone());
        tokio::task::spawn_blocking(move || state::load(&base, &uid))
            .await
            .map_err(|e| Error::Io(format!("state load task failed: {e}")))?
    };
    let union: Vec<i64> = history
        .iter()
        .copied()
        .chain(current_ids.iter().copied())
        .collect::<BTreeSet<i64>>()
        .into_iter()
        .collect();
    {
        let (base, uid, ids) = (client.base_url().to_owned(), uid.clone(), union.clone());
        tokio::task::spawn_blocking(move || state::save(&base, &uid, &ids))
            .await
            .map_err(|e| Error::Io(format!("state save task failed: {e}")))?;
    }

    let past_only: Vec<i64> = history
        .iter()
        .copied()
        .filter(|id| !current_ids.contains(id))
        .collect();

    let non_assignee_filters = make_filters(
        &project_id,
        &status_id,
        params.open,
        &type_id,
        &None,
        &params.subject,
    );

    let mut merged = current;
    for chunk in past_only.chunks(100) {
        let id_values: Vec<Value> = chunk.iter().map(|id| json!(id.to_string())).collect();
        let mut filters = non_assignee_filters.clone();
        filters.push(json!({"id": {"operator": "=", "values": id_values}}));
        let q = vec![("filters".to_string(), filters_json(&filters)?)];
        let extra = client.collect("work_packages", &q).await?;
        merged.extend(extra);
    }

    merged.sort_by(|a, b| {
        let av = a.get("updatedAt").and_then(|v| v.as_str()).unwrap_or("");
        let bv = b.get("updatedAt").and_then(|v| v.as_str()).unwrap_or("");
        bv.cmp(av)
    });
    if let Some(l) = params.limit {
        if l >= 0 && merged.len() > l as usize {
            merged.truncate(l as usize);
        }
    }

    render_elements(client, merged, raw).await
}

/// Run a saved query and return its embedded work packages.
pub async fn query(
    client: &Client,
    query_id: i64,
    offset: i64,
    limit: Option<i64>,
    raw: bool,
) -> Result<Value, Error> {
    let q = paging_query(offset, limit);
    let payload = client
        .request_json_query(
            reqwest::Method::GET,
            &format!("queries/{query_id}"),
            &q,
            None,
        )
        .await?;
    let elements = payload
        .get("_embedded")
        .and_then(|e| e.get("results"))
        .map(embedded_elements)
        .unwrap_or_default();
    render_elements(client, elements, raw).await
}

/// Full-text search across work packages.
pub async fn search(
    client: &Client,
    text: &str,
    offset: i64,
    limit: Option<i64>,
    raw: bool,
) -> Result<Value, Error> {
    let mut q = paging_query(offset, limit);
    let filters = vec![json!({"search": {"operator": "**", "values": [text]}})];
    q.push(("filters".to_string(), filters_json(&filters)?));
    let payload = client
        .request_json_query(reqwest::Method::GET, "work_packages", &q, None)
        .await?;
    let elements = embedded_elements(&payload);
    render_elements(client, elements, raw).await
}

/// Fetch a single work package by id.
pub async fn get(client: &Client, id: i64, raw: bool) -> Result<Value, Error> {
    let payload = client
        .request_json(reqwest::Method::GET, &format!("work_packages/{id}"), None)
        .await?;
    if raw {
        Ok(payload)
    } else {
        with_custom_fields(client, &payload).await
    }
}

/// Create a work package.
pub async fn create(client: &Client, fields: WpFields, raw: bool) -> Result<Value, Error> {
    let body = build_links_and_body(client, true, &fields).await?;
    let payload = client.post_json("work_packages", body).await?;
    if raw {
        Ok(payload)
    } else {
        Ok(normalize::work_package(&payload))
    }
}

/// Update a work package, carrying its current `lockVersion`.
pub async fn update(client: &Client, id: i64, fields: WpFields, raw: bool) -> Result<Value, Error> {
    let current = client
        .request_json(reqwest::Method::GET, &format!("work_packages/{id}"), None)
        .await?;
    let lock_version = current.get("lockVersion").cloned().unwrap_or(Value::Null);
    let mut body = build_links_and_body(client, false, &fields).await?;
    if let Some(map) = body.as_object_mut() {
        map.insert("lockVersion".to_string(), lock_version);
    }
    let payload = client
        .patch_json(&format!("work_packages/{id}"), body)
        .await?;
    if raw {
        Ok(payload)
    } else {
        Ok(normalize::work_package(&payload))
    }
}

/// Delete a work package.
pub async fn delete(client: &Client, id: i64) -> Result<Value, Error> {
    client.delete(&format!("work_packages/{id}")).await?;
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

    fn wp_element(id: i64) -> Value {
        json!({
            "id": id,
            "subject": "S",
            "updatedAt": "2026-01-01T00:00:00Z",
            "_links": {"status": {"title": "New"}}
        })
    }

    #[tokio::test]
    async fn list_subject_filter_normalizes_with_custom_fields() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/work_packages"))
            .and(query_param(
                "filters",
                "[{\"subject\":{\"operator\":\"~\",\"values\":[\"hello\"]}}]",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_embedded": {"elements": [wp_element(1)]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "wp-list-subject");
        let params = WpListParams {
            subject: Some("hello".to_string()),
            ..Default::default()
        };
        let out = list(&c, params, false).await.unwrap();
        let arr = out.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], json!(1));
        assert!(arr[0].get("customFields").is_some());
    }

    #[tokio::test]
    async fn list_page_returns_reported_total() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/work_packages"))
            .and(query_param("pageSize", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total": 5,
                "_embedded": {"elements": [wp_element(1), wp_element(2)]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "wp-list-page");
        let params = WpListParams {
            offset: 1,
            limit: Some(2),
            ..Default::default()
        };
        let (page, total) = list_page(&c, params, true).await.unwrap();
        assert_eq!(total, 5);
        assert_eq!(page.as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn list_open_uses_o_operator() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/work_packages"))
            .and(query_param(
                "filters",
                "[{\"status_id\":{\"operator\":\"o\",\"values\":[]}}]",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_embedded": {"elements": [wp_element(2)]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "wp-list-open");
        let params = WpListParams {
            open: true,
            ..Default::default()
        };
        let out = list(&c, params, true).await.unwrap();
        assert_eq!(out.as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn search_uses_double_star_operator() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/work_packages"))
            .and(query_param(
                "filters",
                "[{\"search\":{\"operator\":\"**\",\"values\":[\"foo\"]}}]",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_embedded": {"elements": [wp_element(3)]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "wp-search");
        let out = search(&c, "foo", 1, None, false).await.unwrap();
        assert_eq!(out.as_array().unwrap()[0]["id"], json!(3));
    }

    #[tokio::test]
    async fn get_normalizes_and_adds_custom_fields() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/work_packages/7"))
            .respond_with(ResponseTemplate::new(200).set_body_json(wp_element(7)))
            .mount(&server)
            .await;
        let c = client_for(&server, "wp-get");
        let out = get(&c, 7, false).await.unwrap();
        assert_eq!(out["id"], json!(7));
        assert_eq!(out["customFields"], json!([]));
    }

    #[tokio::test]
    async fn create_builds_links_and_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/projects/proj1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": 7})))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v3/work_packages"))
            .respond_with(ResponseTemplate::new(201).set_body_json(wp_element(10)))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "wp-create");
        let fields = WpFields {
            subject: Some("New WP".to_string()),
            description: Some("Body".to_string()),
            project: Some("proj1".to_string()),
            type_: Some("1".to_string()),
            ..Default::default()
        };
        let out = create(&c, fields, false).await.unwrap();
        assert_eq!(out["id"], json!(10));

        let requests = server.received_requests().await.unwrap();
        let post = requests
            .iter()
            .find(|r| r.method == wiremock::http::Method::POST)
            .unwrap();
        let body: Value = serde_json::from_slice(&post.body).unwrap();
        assert_eq!(body["subject"], json!("New WP"));
        assert_eq!(
            body["_links"]["project"]["href"],
            json!("/api/v3/projects/7")
        );
        assert_eq!(body["_links"]["type"]["href"], json!("/api/v3/types/1"));
    }

    #[tokio::test]
    async fn update_reads_lock_version_and_patches() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/work_packages/5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 5, "lockVersion": 3
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("PATCH"))
            .and(path("/api/v3/work_packages/5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(wp_element(5)))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "wp-update");
        let fields = WpFields {
            description: Some("Updated".to_string()),
            ..Default::default()
        };
        let out = update(&c, 5, fields, false).await.unwrap();
        assert_eq!(out["id"], json!(5));

        let requests = server.received_requests().await.unwrap();
        let patch = requests
            .iter()
            .find(|r| r.method == wiremock::http::Method::PATCH)
            .unwrap();
        let body: Value = serde_json::from_slice(&patch.body).unwrap();
        assert_eq!(body["lockVersion"], json!(3));
        assert_eq!(body["description"], json!({"raw": "Updated"}));
    }

    #[tokio::test]
    async fn delete_returns_deleted_id() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v3/work_packages/9"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "wp-delete");
        let out = delete(&c, 9).await.unwrap();
        assert_eq!(out, json!({"deleted": 9}));
    }

    #[tokio::test]
    async fn include_past_without_assignee_is_usage_error() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("OPENPROJECT_STATE", tmp.path().join("history.json"));
        let server = MockServer::start().await;
        let c = client_for(&server, "wp-include-past-noassignee");
        let params = WpListParams {
            include_past: true,
            ..Default::default()
        };
        let err = list(&c, params, false).await.unwrap_err();
        assert_eq!(err.exit_code(), 2);
        std::env::remove_var("OPENPROJECT_STATE");
    }
}
