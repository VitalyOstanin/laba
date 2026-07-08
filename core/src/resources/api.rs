//! Raw API passthrough (`api` command), mirroring `gh api`: issue an arbitrary
//! request against the OpenProject API and return the parsed JSON response.
//! Ported 1:1 from the predecessor Python `api.py`. Field parsing of `-f`/`-F`
//! into pairs is done by the CLI; here the already-parsed slices are accepted.

use serde_json::{Map, Value};

use crate::client::Client;
use crate::error::Error;

/// Methods that carry no request body; their fields become query parameters.
const BODILESS: &[&str] = &["GET", "HEAD", "DELETE"];

/// Perform a raw API call. `fields` are string-valued form fields; `raw_fields`
/// are typed fields (parsed as JSON with a string fallback) that overwrite any
/// same-named string field. `input`, when set, is a ready JSON string used as the
/// request body (fields are then ignored for the body).
pub async fn call(
    client: &Client,
    method: &str,
    path: &str,
    fields: &[(String, String)],
    raw_fields: &[(String, String)],
    input: Option<&str>,
) -> Result<Value, Error> {
    let upper = method.to_uppercase();
    let bodiless = BODILESS.contains(&upper.as_str());

    let mut map: Map<String, Value> = Map::new();
    for (k, v) in fields {
        map.insert(k.clone(), Value::String(v.clone()));
    }
    for (k, v) in raw_fields {
        let parsed = serde_json::from_str(v).unwrap_or_else(|_| Value::String(v.clone()));
        map.insert(k.clone(), parsed);
    }

    let mut query: Vec<(String, String)> = Vec::new();
    let mut body: Option<Value> = None;

    if let Some(text) = input {
        body = Some(
            serde_json::from_str(text)
                .map_err(|_| Error::Usage("--input is not valid JSON".into()))?,
        );
    } else if !map.is_empty() {
        if bodiless {
            for (k, v) in &map {
                let s = match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                query.push((k.clone(), s));
            }
        } else {
            body = Some(Value::Object(map));
        }
    }

    let m = reqwest::Method::from_bytes(upper.as_bytes())
        .map_err(|_| Error::Usage(format!("invalid method '{method}'")))?;
    client.request_json_query(m, path, &query, body).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerProfile;
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn profile_for(url: &str) -> ServerProfile {
        ServerProfile {
            backend: Default::default(),
            base_url: url.into(),
            timeout: 30,
            verify_ssl: true,
            proxy: None,
        }
    }

    fn client_for(server: &MockServer, name: &str) -> Client {
        Client::new(name, &profile_for(&server.uri()), "t".into(), Some("none")).unwrap()
    }

    fn pair(k: &str, v: &str) -> (String, String) {
        (k.to_string(), v.to_string())
    }

    #[tokio::test]
    async fn get_with_fields_sends_query() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/projects"))
            .and(query_param("name", "core"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "api-get-fields");
        let out = call(&c, "get", "projects", &[pair("name", "core")], &[], None)
            .await
            .unwrap();
        assert_eq!(out, json!({"ok": true}));
    }

    #[tokio::test]
    async fn post_with_raw_fields_sends_typed_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v3/things"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id": 1})))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "api-post-raw");
        let out = call(&c, "post", "things", &[], &[pair("count", "3")], None)
            .await
            .unwrap();
        assert_eq!(out, json!({"id": 1}));

        let requests = server.received_requests().await.unwrap();
        let post = requests
            .iter()
            .find(|r| r.method == wiremock::http::Method::POST)
            .unwrap();
        let body: Value = serde_json::from_slice(&post.body).unwrap();
        assert_eq!(body["count"], json!(3));
        assert!(body["count"].is_number());
    }

    #[tokio::test]
    async fn input_valid_json_becomes_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v3/things"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id": 2})))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "api-input");
        let out = call(
            &c,
            "post",
            "things",
            &[pair("ignored", "x")],
            &[],
            Some(r#"{"subject": "hi"}"#),
        )
        .await
        .unwrap();
        assert_eq!(out, json!({"id": 2}));

        let requests = server.received_requests().await.unwrap();
        let post = requests
            .iter()
            .find(|r| r.method == wiremock::http::Method::POST)
            .unwrap();
        let body: Value = serde_json::from_slice(&post.body).unwrap();
        assert_eq!(body, json!({"subject": "hi"}));
    }

    #[tokio::test]
    async fn input_invalid_json_is_usage_error() {
        let server = MockServer::start().await;
        let c = client_for(&server, "api-input-bad");
        let err = call(&c, "post", "things", &[], &[], Some("not json"))
            .await
            .unwrap_err();
        assert_eq!(err.exit_code(), 2);
    }

    #[tokio::test]
    async fn delete_with_fields_sends_query() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v3/things/1"))
            .and(query_param("force", "true"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "api-delete-fields");
        let out = call(
            &c,
            "delete",
            "things/1",
            &[pair("force", "true")],
            &[],
            None,
        )
        .await
        .unwrap();
        assert_eq!(out, Value::Null);
    }
}
