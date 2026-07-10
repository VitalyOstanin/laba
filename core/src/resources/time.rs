//! Time entry resource operations (`time` command family): list, get, create,
//! update, delete. Ported from the predecessor Python `time.py`, extended with
//! `--duration` (human durations) and a `--comment` on create/update.

use serde_json::{json, Map, Value};

use crate::client::Client;
use crate::duration::{hours_to_iso8601, parse_human_duration};
use crate::error::Error;
use crate::{normalize, resolve};

/// Pagination query pairs: always `offset`, plus `pageSize` when a limit is set.
fn paging_query(offset: i64, limit: Option<i64>) -> Vec<(String, String)> {
    let mut q = vec![("offset".to_string(), offset.to_string())];
    if let Some(l) = limit {
        q.push(("pageSize".to_string(), l.to_string()));
    }
    q
}

/// Serialize a filter array to the compact JSON string the API expects.
fn filters_json(filters: &[Value]) -> Result<String, Error> {
    serde_json::to_string(&Value::Array(filters.to_vec()))
        .map_err(|e| Error::Internal(format!("encode filters: {e}")))
}

/// Resolve the logged hours for create: from `--duration` or `--hours`. Exactly
/// one is required; passing both is a usage error.
fn resolve_hours(hours: Option<f64>, duration: Option<&str>) -> Result<f64, Error> {
    match (hours, duration) {
        (Some(_), Some(_)) => Err(Error::Usage(
            "pass either --hours or --duration, not both".to_string(),
        )),
        (_, Some(d)) => parse_human_duration(d),
        (Some(h), None) => Ok(h),
        (None, None) => Err(Error::Usage(
            "time create requires --hours or --duration".to_string(),
        )),
    }
}

/// Resolve the optional logged hours for update: `None` means leave the field
/// unchanged. Passing both `--hours` and `--duration` is a usage error.
fn resolve_hours_update(hours: Option<f64>, duration: Option<&str>) -> Result<Option<f64>, Error> {
    match (hours, duration) {
        (Some(_), Some(_)) => Err(Error::Usage(
            "pass either --hours or --duration, not both".to_string(),
        )),
        (_, Some(d)) => Ok(Some(parse_human_duration(d)?)),
        (Some(h), None) => Ok(Some(h)),
        (None, None) => Ok(None),
    }
}

/// Today's local date formatted as `YYYY-MM-DD`.
fn today_local() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Build the time-entry filter array shared by [`list`] and [`list_all`].
async fn build_filters(
    client: &Client,
    user: Option<&str>,
    project: Option<&str>,
    work_package: Option<i64>,
    since: Option<&str>,
    until: Option<&str>,
) -> Result<Vec<Value>, Error> {
    let mut filters: Vec<Value> = Vec::new();
    if let Some(p) = project {
        let pid = resolve::project_id(client, p).await?;
        filters.push(json!({"project": {"operator": "=", "values": [pid]}}));
    }
    if let Some(wp) = work_package {
        filters.push(json!({"workPackage": {"operator": "=", "values": [wp.to_string()]}}));
    }
    if let Some(u) = user {
        let uid = resolve::resolve_principal_id(client, u).await?;
        filters.push(json!({"user": {"operator": "=", "values": [uid]}}));
    }
    if since.is_some() || until.is_some() {
        filters.push(json!({"spentOn": {"operator": "<>d", "values": [
            since.unwrap_or(""),
            until.unwrap_or(""),
        ]}}));
    }
    Ok(filters)
}

/// Render collected elements to normalized time entries unless `raw`.
fn render(elements: Vec<Value>, raw: bool) -> Value {
    if raw {
        return Value::Array(elements);
    }
    Value::Array(elements.iter().map(normalize::time_entry).collect())
}

/// List time entries, optionally filtered by user, project, work package and a
/// spent-on date range. Returns a single page (`offset`/`limit`); callers that
/// need the full set (e.g. timelog aggregation) must use [`list_all`].
#[allow(clippy::too_many_arguments)]
pub async fn list(
    client: &Client,
    user: Option<&str>,
    project: Option<&str>,
    work_package: Option<i64>,
    since: Option<&str>,
    until: Option<&str>,
    offset: i64,
    limit: Option<i64>,
    raw: bool,
) -> Result<Value, Error> {
    let filters = build_filters(client, user, project, work_package, since, until).await?;
    let mut q = paging_query(offset, limit);
    if !filters.is_empty() {
        q.push(("filters".to_string(), filters_json(&filters)?));
    }
    let payload = client
        .request_json_query(reqwest::Method::GET, "time_entries", &q, None)
        .await?;
    Ok(render(normalize::collection(&payload), raw))
}

/// List every time entry matching the filters, following pagination across all
/// pages. The timelog aggregation needs this: a single page silently undercounts
/// logged time and produces false deficits.
pub async fn list_all(
    client: &Client,
    user: Option<&str>,
    project: Option<&str>,
    work_package: Option<i64>,
    since: Option<&str>,
    until: Option<&str>,
    raw: bool,
) -> Result<Value, Error> {
    let filters = build_filters(client, user, project, work_package, since, until).await?;
    let mut q: Vec<(String, String)> = Vec::new();
    if !filters.is_empty() {
        q.push(("filters".to_string(), filters_json(&filters)?));
    }
    let elements = client.collect("time_entries", &q).await?;
    Ok(render(elements, raw))
}

/// List the available time-entry activity types (`{id, name}`), for pickers.
/// The activities endpoint is not paginated.
pub async fn list_activities(client: &Client) -> Result<Value, Error> {
    let payload = client
        .request_json(reqwest::Method::GET, "time_entries/activities", None)
        .await?;
    let out: Vec<Value> = normalize::collection(&payload)
        .iter()
        .map(|e| json!({"id": e.get("id").cloned().unwrap_or(Value::Null), "name": e.get("name").cloned().unwrap_or(Value::Null)}))
        .collect();
    Ok(Value::Array(out))
}

/// Fetch a single time entry by id.
pub async fn get(client: &Client, id: i64, raw: bool) -> Result<Value, Error> {
    let payload = client
        .request_json(reqwest::Method::GET, &format!("time_entries/{id}"), None)
        .await?;
    if raw {
        Ok(payload)
    } else {
        Ok(normalize::time_entry(&payload))
    }
}

/// Create a time entry against a work package.
#[allow(clippy::too_many_arguments)]
pub async fn create(
    client: &Client,
    work_package: i64,
    hours: Option<f64>,
    duration: Option<&str>,
    spent_on: Option<&str>,
    comment: Option<&str>,
    activity: Option<&str>,
    raw: bool,
) -> Result<Value, Error> {
    let h = resolve_hours(hours, duration)?;
    let spent = match spent_on {
        Some(s) => s.to_string(),
        None => today_local(),
    };
    let mut body = json!({
        "hours": hours_to_iso8601(h)?,
        "spentOn": spent,
        "_links": {"workPackage": {"href": format!("/api/v3/work_packages/{work_package}")}},
    });
    if let Some(c) = comment {
        body["comment"] = json!({"raw": c});
    }
    if let Some(a) = activity {
        let aid = resolve::activity_id(client, a).await?;
        body["_links"]["activity"] =
            json!({"href": format!("/api/v3/time_entries/activities/{aid}")});
    }
    let payload = client.post_json("time_entries", body).await?;
    if raw {
        Ok(payload)
    } else {
        Ok(normalize::time_entry(&payload))
    }
}

/// Update an existing time entry, carrying its current `lockVersion`.
#[allow(clippy::too_many_arguments)]
pub async fn update(
    client: &Client,
    id: i64,
    hours: Option<f64>,
    duration: Option<&str>,
    spent_on: Option<&str>,
    comment: Option<&str>,
    activity: Option<&str>,
    raw: bool,
) -> Result<Value, Error> {
    let current = client
        .request_json(reqwest::Method::GET, &format!("time_entries/{id}"), None)
        .await?;
    let lock_version = current.get("lockVersion").cloned().unwrap_or(Value::Null);
    let mut body = json!({"lockVersion": lock_version});

    if let Some(h) = resolve_hours_update(hours, duration)? {
        body["hours"] = json!(hours_to_iso8601(h)?);
    }
    if let Some(s) = spent_on {
        body["spentOn"] = json!(s);
    }
    if let Some(c) = comment {
        body["comment"] = json!({"raw": c});
    }
    let mut links: Map<String, Value> = Map::new();
    if let Some(a) = activity {
        let aid = resolve::activity_id(client, a).await?;
        links.insert(
            "activity".to_string(),
            json!({"href": format!("/api/v3/time_entries/activities/{aid}")}),
        );
    }
    if !links.is_empty() {
        body["_links"] = Value::Object(links);
    }

    let payload = client
        .patch_json(&format!("time_entries/{id}"), body)
        .await?;
    if raw {
        Ok(payload)
    } else {
        Ok(normalize::time_entry(&payload))
    }
}

/// Delete a time entry.
pub async fn delete(client: &Client, id: i64) -> Result<Value, Error> {
    client.delete(&format!("time_entries/{id}")).await?;
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
            display_fields: Vec::new(),
        }
    }

    fn client_for(server: &MockServer, name: &str) -> Client {
        Client::new(name, &profile_for(&server.uri()), "t".into(), Some("none")).unwrap()
    }

    fn te_element(id: i64) -> Value {
        json!({
            "id": id,
            "hours": "PT1H30M",
            "spentOn": "2026-01-01",
            "comment": {"raw": "c"},
        })
    }

    async fn last_post_body(server: &MockServer) -> Value {
        let requests = server.received_requests().await.unwrap();
        let post = requests
            .iter()
            .find(|r| r.method == wiremock::http::Method::POST)
            .unwrap();
        serde_json::from_slice(&post.body).unwrap()
    }

    fn mock_create(server: &MockServer) -> impl std::future::Future<Output = ()> + '_ {
        Mock::given(method("POST"))
            .and(path("/api/v3/time_entries"))
            .respond_with(ResponseTemplate::new(201).set_body_json(te_element(1)))
            .mount(server)
    }

    #[tokio::test]
    async fn list_all_follows_pagination() {
        let server = MockServer::start().await;
        // Page 1 reports total 3 and returns two entries; page 2 returns the rest.
        Mock::given(method("GET"))
            .and(path("/api/v3/time_entries"))
            .and(query_param("offset", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total": 3,
                "_embedded": {"elements": [te_element(1), te_element(2)]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v3/time_entries"))
            .and(query_param("offset", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total": 3,
                "_embedded": {"elements": [te_element(3)]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "te-list-all");
        let out = list_all(&c, None, None, None, None, None, true)
            .await
            .unwrap();
        // All three entries across both pages are collected, not just page one.
        assert_eq!(out.as_array().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn create_with_duration_formats_iso_hours() {
        let server = MockServer::start().await;
        mock_create(&server).await;
        let c = client_for(&server, "te-create-dur");
        create(
            &c,
            5,
            None,
            Some("90m"),
            Some("2026-01-01"),
            None,
            None,
            false,
        )
        .await
        .unwrap();
        let body = last_post_body(&server).await;
        assert_eq!(body["hours"], json!("PT1H30M"));
        assert_eq!(
            body["_links"]["workPackage"]["href"],
            json!("/api/v3/work_packages/5")
        );
    }

    #[tokio::test]
    async fn create_with_hours_formats_iso_hours() {
        let server = MockServer::start().await;
        mock_create(&server).await;
        let c = client_for(&server, "te-create-hours");
        create(
            &c,
            5,
            Some(1.5),
            None,
            Some("2026-01-01"),
            None,
            None,
            false,
        )
        .await
        .unwrap();
        let body = last_post_body(&server).await;
        assert_eq!(body["hours"], json!("PT1H30M"));
    }

    #[tokio::test]
    async fn create_without_hours_or_duration_is_usage_error() {
        let server = MockServer::start().await;
        let c = client_for(&server, "te-create-none");
        let err = create(&c, 5, None, None, None, None, None, false)
            .await
            .unwrap_err();
        assert_eq!(err.exit_code(), 2);
    }

    #[tokio::test]
    async fn create_with_both_hours_and_duration_is_usage_error() {
        let server = MockServer::start().await;
        let c = client_for(&server, "te-create-both");
        let err = create(&c, 5, Some(1.0), Some("2h"), None, None, None, false)
            .await
            .unwrap_err();
        assert_eq!(err.exit_code(), 2);
    }

    #[tokio::test]
    async fn create_with_comment_sends_raw_object() {
        let server = MockServer::start().await;
        mock_create(&server).await;
        let c = client_for(&server, "te-create-comment");
        create(
            &c,
            5,
            Some(1.0),
            None,
            Some("2026-01-01"),
            Some("note"),
            None,
            false,
        )
        .await
        .unwrap();
        let body = last_post_body(&server).await;
        assert_eq!(body["comment"], json!({"raw": "note"}));
    }

    #[tokio::test]
    async fn create_with_activity_resolves_and_links() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/time_entries/activities"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total": 1,
                "_embedded": {"elements": [{"id": 3, "name": "Development"}]}
            })))
            .mount(&server)
            .await;
        mock_create(&server).await;
        let c = client_for(&server, "te-create-activity");
        create(
            &c,
            5,
            Some(1.0),
            None,
            Some("2026-01-01"),
            None,
            Some("Development"),
            false,
        )
        .await
        .unwrap();
        let body = last_post_body(&server).await;
        assert_eq!(
            body["_links"]["activity"]["href"],
            json!("/api/v3/time_entries/activities/3")
        );
    }

    #[tokio::test]
    async fn create_defaults_spent_on_to_today() {
        let server = MockServer::start().await;
        mock_create(&server).await;
        let c = client_for(&server, "te-create-today");
        create(&c, 5, Some(1.0), None, None, None, None, false)
            .await
            .unwrap();
        let body = last_post_body(&server).await;
        let spent = body["spentOn"].as_str().unwrap();
        assert!(
            spent.len() == 10
                && spent.as_bytes()[4] == b'-'
                && spent.as_bytes()[7] == b'-'
                && spent[..4].bytes().all(|b| b.is_ascii_digit()),
            "spentOn not YYYY-MM-DD: {spent}"
        );
    }

    #[tokio::test]
    async fn list_activities_returns_id_and_name() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/time_entries/activities"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_embedded": {"elements": [
                    {"id": 1, "name": "Development"},
                    {"id": 2, "name": "Testing"}
                ]}
            })))
            .mount(&server)
            .await;
        let c = client_for(&server, "te-activities");
        let out = list_activities(&c).await.unwrap();
        let arr = out.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0], json!({"id": 1, "name": "Development"}));
    }

    #[tokio::test]
    async fn list_date_range_uses_between_operator() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/time_entries"))
            .and(query_param(
                "filters",
                "[{\"spentOn\":{\"operator\":\"<>d\",\"values\":[\"2026-01-01\",\"2026-01-31\"]}}]",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_embedded": {"elements": [te_element(1)]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "te-list-range");
        let out = list(
            &c,
            None,
            None,
            None,
            Some("2026-01-01"),
            Some("2026-01-31"),
            1,
            None,
            false,
        )
        .await
        .unwrap();
        assert_eq!(out.as_array().unwrap()[0]["id"], json!(1));
    }

    #[tokio::test]
    async fn list_user_filter_resolves_principal() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/time_entries"))
            .and(query_param(
                "filters",
                "[{\"user\":{\"operator\":\"=\",\"values\":[\"42\"]}}]",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_embedded": {"elements": [te_element(2)]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "te-list-user");
        let out = list(&c, Some("42"), None, None, None, None, 1, None, false)
            .await
            .unwrap();
        assert_eq!(out.as_array().unwrap()[0]["id"], json!(2));
    }

    #[tokio::test]
    async fn update_always_sends_lock_version() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/time_entries/7"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 7, "lockVersion": 4
            })))
            .mount(&server)
            .await;
        Mock::given(method("PATCH"))
            .and(path("/api/v3/time_entries/7"))
            .respond_with(ResponseTemplate::new(200).set_body_json(te_element(7)))
            .mount(&server)
            .await;
        let c = client_for(&server, "te-update-lock");
        update(&c, 7, None, None, Some("2026-02-02"), None, None, false)
            .await
            .unwrap();
        let requests = server.received_requests().await.unwrap();
        let patch = requests
            .iter()
            .find(|r| r.method == wiremock::http::Method::PATCH)
            .unwrap();
        let body: Value = serde_json::from_slice(&patch.body).unwrap();
        assert_eq!(body["lockVersion"], json!(4));
        // hours untouched when neither hours nor duration is given.
        assert!(body.get("hours").is_none());
        assert_eq!(body["spentOn"], json!("2026-02-02"));
    }
}
