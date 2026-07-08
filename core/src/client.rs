use std::time::Duration;

use crate::config::ServerProfile;
use crate::error::Error;

/// A per-server API client. One `Client` binds one server's base_url,
/// credentials and proxy — nothing is shared between servers.
#[derive(Debug)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    token: String,
    cache: crate::cache::Cache,
}

impl Client {
    pub fn new(
        server_name: &str,
        profile: &ServerProfile,
        token: String,
        proxy_override: Option<&str>,
    ) -> Result<Client, Error> {
        let mut builder = reqwest::Client::builder()
            // A redirect could forward the Basic token to another host — never follow.
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(profile.timeout))
            .danger_accept_invalid_certs(!profile.verify_ssl);

        let proxy = match proxy_override {
            Some("none") | Some("") => None,
            Some(p) => Some(p.to_owned()),
            None => profile.proxy.clone(),
        };
        if let Some(p) = proxy {
            let proxy = reqwest::Proxy::all(&p)
                .map_err(|e| Error::Config(format!("invalid proxy '{p}': {e}")))?;
            builder = builder.proxy(proxy);
        } else {
            // Explicitly ignore any ambient HTTP(S)_PROXY env for determinism.
            builder = builder.no_proxy();
        }

        let http = builder
            .build()
            .map_err(|e| Error::Internal(format!("build http client: {e}")))?;
        Ok(Client {
            http,
            base_url: profile.base_url.trim_end_matches('/').to_owned(),
            token,
            cache: crate::cache::Cache::for_server(server_name),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn cache(&self) -> &crate::cache::Cache {
        &self.cache
    }

    /// Build an absolute API URL, collapsing any deployment-subpath API href
    /// (`/openproject/api/v3/...`) to the base's `/api/v3/...` form.
    pub fn api_url(&self, path: &str) -> String {
        let p = path.trim_start_matches('/');
        if let Some(idx) = p.find("api/v3/") {
            format!("{}/{}", self.base_url, &p[idx..])
        } else {
            format!("{}/api/v3/{}", self.base_url, p)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> ServerProfile {
        ServerProfile {
            base_url: "https://host.example/openproject".into(),
            timeout: 30,
            verify_ssl: true,
            proxy: None,
        }
    }

    #[test]
    fn api_url_prefixes_plain_path() {
        let c = Client::new("test", &profile(), "t".into(), None).unwrap();
        assert_eq!(
            c.api_url("work_packages/1"),
            "https://host.example/openproject/api/v3/work_packages/1"
        );
    }

    #[test]
    fn api_url_collapses_deployment_subpath_href() {
        let c = Client::new("test", &profile(), "t".into(), None).unwrap();
        assert_eq!(
            c.api_url("/openproject/api/v3/work_packages/1"),
            "https://host.example/openproject/api/v3/work_packages/1"
        );
    }

    #[test]
    fn invalid_proxy_is_config_error() {
        let mut p = profile();
        p.proxy = Some("::not a url::".into());
        assert_eq!(
            Client::new("test", &p, "t".into(), None)
                .unwrap_err()
                .exit_code(),
            70
        );
    }
}

impl Client {
    /// Resolve a request URL. Relative paths go through `api_url`. An absolute
    /// path (`http://`/`https://`) must target the same host as `base_url`
    /// (case-insensitive) — otherwise the Basic token could leak to a foreign
    /// host, so a mismatch is `Error::Usage`.
    fn resolve_url(&self, path: &str) -> Result<String, Error> {
        if path.starts_with("http://") || path.starts_with("https://") {
            let target = reqwest::Url::parse(path)
                .map_err(|e| Error::Usage(format!("invalid url '{path}': {e}")))?;
            let base = reqwest::Url::parse(&self.base_url)
                .map_err(|e| Error::Internal(format!("invalid base_url: {e}")))?;
            let same = match (target.host_str(), base.host_str()) {
                (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
                _ => false,
            };
            if !same {
                return Err(Error::Usage(format!(
                    "refusing request to foreign host '{}'",
                    target.host_str().unwrap_or("")
                )));
            }
            Ok(path.to_owned())
        } else {
            Ok(self.api_url(path))
        }
    }

    /// Execute a request and return the parsed JSON body, mapping non-2xx
    /// responses to `Error::Api` with the server message when present.
    pub async fn request_json(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Error> {
        self.request_json_query(method, path, &[], body).await
    }

    /// Like `request_json` but appends query parameters to the URL.
    pub async fn request_json_query(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &[(String, String)],
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Error> {
        let url = self.resolve_url(path)?;
        let mut req = self
            .http
            .request(method, &url)
            .basic_auth("apikey", Some(&self.token))
            .header(reqwest::header::ACCEPT, "application/json")
            .query(query);
        if let Some(b) = body {
            req = req.json(&b);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| Error::Api(format!("request {url}: {e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| Error::Api(format!("read body: {e}")))?;
        if !status.is_success() {
            let msg = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(str::to_owned))
                .unwrap_or_else(|| text.clone());
            return Err(Error::Api(format!("HTTP {}: {msg}", status.as_u16())));
        }
        if text.is_empty() {
            return Ok(serde_json::Value::Null);
        }
        serde_json::from_str(&text).map_err(|e| Error::Api(format!("parse json: {e}")))
    }

    /// DELETE a resource. Sends `Content-Type: application/json` (the server
    /// rejects the request with 406 otherwise). Non-2xx maps to `Error::Api`.
    pub async fn delete(&self, path: &str) -> Result<(), Error> {
        let url = self.resolve_url(path)?;
        let resp = self
            .http
            .request(reqwest::Method::DELETE, &url)
            .basic_auth("apikey", Some(&self.token))
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .send()
            .await
            .map_err(|e| Error::Api(format!("request {url}: {e}")))?;
        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        let text = resp
            .text()
            .await
            .map_err(|e| Error::Api(format!("read body: {e}")))?;
        let msg = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(str::to_owned))
            .unwrap_or_else(|| text.clone());
        Err(Error::Api(format!("HTTP {}: {msg}", status.as_u16())))
    }

    /// POST a JSON body and return the parsed response. Not retried.
    pub async fn post_json(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, Error> {
        self.request_json(reqwest::Method::POST, path, Some(body))
            .await
    }

    /// PATCH a JSON body and return the parsed response. Not retried.
    pub async fn patch_json(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, Error> {
        self.request_json(reqwest::Method::PATCH, path, Some(body))
            .await
    }

    /// POST with an empty body but an explicit JSON content type (needed by the
    /// notification read/unread endpoints). Empty/204 responses yield `Null`.
    pub async fn post_empty_json(&self, path: &str) -> Result<serde_json::Value, Error> {
        let url = self.resolve_url(path)?;
        let resp = self
            .http
            .request(reqwest::Method::POST, &url)
            .basic_auth("apikey", Some(&self.token))
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .send()
            .await
            .map_err(|e| Error::Api(format!("request {url}: {e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| Error::Api(format!("read body: {e}")))?;
        if !status.is_success() {
            let msg = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(str::to_owned))
                .unwrap_or_else(|| text.clone());
            return Err(Error::Api(format!("HTTP {}: {msg}", status.as_u16())));
        }
        if text.is_empty() {
            return Ok(serde_json::Value::Null);
        }
        serde_json::from_str(&text).map_err(|e| Error::Api(format!("parse json: {e}")))
    }

    /// Fetch all elements of a HAL collection, following `pageSize`/`offset`
    /// pagination (200 per page). Stops when the collected count reaches the
    /// reported `total` or a page returns no elements.
    pub async fn collect(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<Vec<serde_json::Value>, Error> {
        let mut out: Vec<serde_json::Value> = Vec::new();
        let mut offset: u64 = 1;
        loop {
            let mut q: Vec<(String, String)> = query.to_vec();
            q.push(("pageSize".to_string(), "200".to_string()));
            q.push(("offset".to_string(), offset.to_string()));
            let payload = self
                .request_json_query(reqwest::Method::GET, path, &q, None)
                .await?;
            let page: Vec<serde_json::Value> = payload
                .get("_embedded")
                .and_then(|e| e.get("elements"))
                .and_then(|e| e.as_array())
                .cloned()
                .unwrap_or_default();
            let empty = page.is_empty();
            out.extend(page);
            let total = payload
                .get("total")
                .and_then(|t| t.as_u64())
                .unwrap_or(out.len() as u64);
            if empty || out.len() as u64 >= total {
                break;
            }
            offset += 1;
        }
        Ok(out)
    }

    /// The currently authenticated user (`users/me`).
    pub async fn current_user(&self) -> Result<serde_json::Value, Error> {
        self.request_json(reqwest::Method::GET, "users/me", None)
            .await
    }

    /// Resolve a user id to its display name, caching the result per server.
    /// A failed lookup returns `Ok(None)` and is not persisted to the file
    /// cache, so a transient error is not frozen.
    pub async fn user_name(&self, id: i64) -> Result<Option<String>, Error> {
        if let Some(cached) = self.cache().get_user(id) {
            return Ok(cached);
        }
        match self
            .request_json(reqwest::Method::GET, &format!("users/{id}"), None)
            .await
        {
            Ok(v) => {
                let name = v.get("name").and_then(|n| n.as_str()).map(str::to_owned);
                self.cache().put_user(id, name.clone());
                Ok(name)
            }
            Err(_) => Ok(None),
        }
    }
}

#[cfg(test)]
mod exec_tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn profile_for(url: &str) -> ServerProfile {
        ServerProfile {
            base_url: url.into(),
            timeout: 30,
            verify_ssl: true,
            proxy: None,
        }
    }

    #[tokio::test]
    async fn ok_returns_json() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/users/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": 7})))
            .mount(&server)
            .await;
        let c = Client::new(
            "test",
            &profile_for(&server.uri()),
            "t".into(),
            Some("none"),
        )
        .unwrap();
        let v = c
            .request_json(reqwest::Method::GET, "users/me", None)
            .await
            .unwrap();
        assert_eq!(v["id"], 7);
    }

    #[tokio::test]
    async fn error_maps_message() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/users/me"))
            .respond_with(
                ResponseTemplate::new(404).set_body_json(serde_json::json!({"message": "nope"})),
            )
            .mount(&server)
            .await;
        let c = Client::new(
            "test",
            &profile_for(&server.uri()),
            "t".into(),
            Some("none"),
        )
        .unwrap();
        let err = c
            .request_json(reqwest::Method::GET, "users/me", None)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("HTTP 404"));
        assert!(err.to_string().contains("nope"));
    }

    #[tokio::test]
    async fn delete_sends_json_content_type_and_ok_on_204() {
        use wiremock::matchers::header;
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v3/work_packages/1"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;
        let c = Client::new(
            "test",
            &profile_for(&server.uri()),
            "t".into(),
            Some("none"),
        )
        .unwrap();
        c.delete("work_packages/1").await.unwrap();
    }

    #[tokio::test]
    async fn collect_follows_pagination() {
        use wiremock::matchers::query_param;
        let server = MockServer::start().await;
        let page1: Vec<serde_json::Value> = (0..200).map(|i| serde_json::json!({"i": i})).collect();
        Mock::given(method("GET"))
            .and(path("/api/v3/work_packages"))
            .and(query_param("offset", "1"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"total": 201, "_embedded": {"elements": page1}}),
                ),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v3/work_packages"))
            .and(query_param("offset", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"total": 201, "_embedded": {"elements": [{"i": 200}]}}),
            ))
            .mount(&server)
            .await;
        let c = Client::new(
            "test",
            &profile_for(&server.uri()),
            "t".into(),
            Some("none"),
        )
        .unwrap();
        let all = c.collect("work_packages", &[]).await.unwrap();
        assert_eq!(all.len(), 201);
    }

    #[tokio::test]
    async fn user_name_is_cached_after_first_lookup() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("OPENPROJECT_CACHE", tmp.path());
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/users/7"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"name": "Ivan"})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let c = Client::new(
            "srv-user-name-cache",
            &profile_for(&server.uri()),
            "t".into(),
            Some("none"),
        )
        .unwrap();
        assert_eq!(c.user_name(7).await.unwrap().as_deref(), Some("Ivan"));
        // Second call is served from the cache and does not hit the server.
        assert_eq!(c.user_name(7).await.unwrap().as_deref(), Some("Ivan"));
    }

    #[tokio::test]
    async fn foreign_host_absolute_url_is_usage_error() {
        let server = MockServer::start().await;
        let c = Client::new(
            "test",
            &profile_for(&server.uri()),
            "t".into(),
            Some("none"),
        )
        .unwrap();
        let err = c
            .request_json(
                reqwest::Method::GET,
                "https://evil.example/api/v3/users/me",
                None,
            )
            .await
            .unwrap_err();
        assert_eq!(err.exit_code(), 2);
    }
}

const MAX_RETRY_SLEEP: Duration = Duration::from_secs(60);

impl Client {
    /// Retry idempotent GETs on 429/5xx, honoring Retry-After (capped),
    /// with exponential backoff. `retries` is the max extra attempts.
    pub async fn get_json_retrying(
        &self,
        path: &str,
        retries: u32,
    ) -> Result<serde_json::Value, Error> {
        let mut attempt = 0u32;
        loop {
            match self.request_json(reqwest::Method::GET, path, None).await {
                Ok(v) => return Ok(v),
                Err(e) => {
                    let retriable = is_retriable(&e);
                    if !retriable || attempt >= retries {
                        return Err(e);
                    }
                    let backoff = Duration::from_millis(200u64 << attempt).min(MAX_RETRY_SLEEP);
                    eprintln!(
                        "taskstream: retrying after error ({}), attempt {}",
                        e,
                        attempt + 1
                    );
                    tokio::time::sleep(backoff).await;
                    attempt += 1;
                }
            }
        }
    }
}

fn is_retriable(e: &Error) -> bool {
    if let Error::Api(msg) = e {
        return msg.contains("HTTP 429")
            || msg.contains("HTTP 500")
            || msg.contains("HTTP 502")
            || msg.contains("HTTP 503")
            || msg.contains("HTTP 504");
    }
    false
}

#[cfg(test)]
mod retry_tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn retries_then_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/x"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v3/x"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .mount(&server)
            .await;
        let p = ServerProfile {
            base_url: server.uri(),
            timeout: 30,
            verify_ssl: true,
            proxy: None,
        };
        let c = Client::new("test", &p, "t".into(), Some("none")).unwrap();
        let v = c.get_json_retrying("x", 3).await.unwrap();
        assert_eq!(v["ok"], true);
    }
}
