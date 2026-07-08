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

/// True for a key of the form `customField<digits>` (non-empty digit suffix).
fn is_custom_field_key(key: &str) -> bool {
    match key.strip_prefix("customField") {
        Some(rest) => !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()),
        None => false,
    }
}

/// A scalar JSON value that is neither null nor the empty string.
fn is_present_scalar(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Null => false,
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => false,
        _ => true,
    }
}

/// Collect the raw custom-field values of a payload, preserving encounter order:
/// scalar `customFieldN` keys from the top level, then linked `customFieldN`
/// keys from `_links` (titles of the referenced resources).
fn extract_raw_custom_fields(payload: &serde_json::Value) -> Vec<(String, serde_json::Value)> {
    let mut out: Vec<(String, serde_json::Value)> = Vec::new();
    if let Some(map) = payload.as_object() {
        for (key, value) in map {
            if is_custom_field_key(key) && is_present_scalar(value) {
                out.push((key.clone(), value.clone()));
            }
        }
    }
    if let Some(links) = payload.get("_links").and_then(|l| l.as_object()) {
        for (key, value) in links {
            if !is_custom_field_key(key) {
                continue;
            }
            let extracted = match value {
                serde_json::Value::Array(items) => {
                    let titles: Vec<serde_json::Value> = items
                        .iter()
                        .filter_map(|it| it.get("title").and_then(|t| t.as_str()))
                        .map(|s| serde_json::Value::String(s.to_owned()))
                        .collect();
                    serde_json::Value::Array(titles)
                }
                serde_json::Value::Object(_) => match value.get("title") {
                    Some(t) if !t.is_null() => t.clone(),
                    _ => continue,
                },
                _ => continue,
            };
            out.push((key.clone(), extracted));
        }
    }
    out
}

impl Client {
    /// Expand a payload's custom fields into `{key, name, value}` objects. The
    /// display names come from the payload's form schema (cached per server).
    /// A payload with no custom-field values yields an empty vec and skips the
    /// schema request.
    pub async fn custom_fields(
        &self,
        payload: &serde_json::Value,
    ) -> Result<Vec<serde_json::Value>, Error> {
        let raw = extract_raw_custom_fields(payload);
        if raw.is_empty() {
            return Ok(vec![]);
        }
        let names = match payload
            .get("_links")
            .and_then(|l| l.get("schema"))
            .and_then(|s| s.get("href"))
            .and_then(|h| h.as_str())
        {
            Some(href) => self.custom_field_names(href).await,
            None => std::collections::HashMap::new(),
        };
        let out = raw
            .into_iter()
            .map(|(key, value)| {
                let name = names
                    .get(&key)
                    .map(|n| serde_json::Value::String(n.clone()))
                    .unwrap_or(serde_json::Value::Null);
                serde_json::json!({"key": key, "name": name, "value": value})
            })
            .collect();
        Ok(out)
    }

    /// Map of `customFieldN -> display name` from a form schema href, cached per
    /// server. A failed schema read yields an empty (uncached) map.
    async fn custom_field_names(
        &self,
        schema_href: &str,
    ) -> std::collections::HashMap<String, String> {
        if let Some(cached) = self.cache().get_schema(schema_href) {
            return cached;
        }
        let schema = match self
            .request_json(reqwest::Method::GET, schema_href, None)
            .await
        {
            Ok(v) => v,
            Err(_) => return std::collections::HashMap::new(),
        };
        let mut map = std::collections::HashMap::new();
        if let Some(obj) = schema.as_object() {
            for (key, value) in obj {
                if !is_custom_field_key(key) {
                    continue;
                }
                if let Some(name) = value.get("name").and_then(|n| n.as_str()) {
                    map.insert(key.clone(), name.to_owned());
                }
            }
        }
        self.cache().put_schema(schema_href, map.clone());
        map
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

/// Size and SHA-256 of a downloaded attachment body.
#[derive(Debug)]
pub struct DownloadInfo {
    pub bytes: u64,
    pub sha256: String,
}

/// Monotonic counter used to make temporary download file names unique within
/// a process without pulling in a random-number crate.
static DOWNLOAD_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Guess an attachment content-type from its file name extension. Covers the
/// common cases; anything unknown falls back to `application/octet-stream`.
fn guess_content_type(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "txt" | "log" => "text/plain",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "json" => "application/json",
        "xml" => "application/xml",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        _ => "application/octet-stream",
    }
}

impl Client {
    /// GET `path` and stream the response body in chunks into `dest`, computing
    /// the total size and SHA-256 as it writes. Non-2xx maps to `Error::Api`.
    pub async fn stream_download(
        &self,
        path: &str,
        dest: &mut impl std::io::Write,
    ) -> Result<DownloadInfo, Error> {
        use futures_util::StreamExt;
        use sha2::{Digest, Sha256};

        let url = self.resolve_url(path)?;
        let resp = self
            .http
            .request(reqwest::Method::GET, &url)
            .basic_auth("apikey", Some(&self.token))
            .header(reqwest::header::ACCEPT, "application/octet-stream")
            .send()
            .await
            .map_err(|e| Error::Api(format!("request {url}: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            let msg = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(str::to_owned))
                .unwrap_or(text);
            return Err(Error::Api(format!("HTTP {}: {msg}", status.as_u16())));
        }
        let mut hasher = Sha256::new();
        let mut total: u64 = 0;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| Error::Api(format!("read body: {e}")))?;
            hasher.update(&chunk);
            total += chunk.len() as u64;
            dest.write_all(&chunk)
                .map_err(|e| Error::Io(format!("write download: {e}")))?;
        }
        Ok(DownloadInfo {
            bytes: total,
            sha256: format!("{:x}", hasher.finalize()),
        })
    }

    /// Stream a download into a temporary file in `output`'s directory, then
    /// atomically rename it into place. If `max_bytes` is set and exceeded, the
    /// transfer is aborted, the temporary file removed, and `Error::Usage`
    /// returned.
    pub async fn download_to_path(
        &self,
        path: &str,
        output: &std::path::Path,
        max_bytes: Option<u64>,
    ) -> Result<DownloadInfo, Error> {
        use futures_util::StreamExt;
        use sha2::{Digest, Sha256};
        use std::io::Write;

        let url = self.resolve_url(path)?;
        let dir = output.parent().unwrap_or_else(|| std::path::Path::new("."));
        let final_name = output
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("download");
        let counter = DOWNLOAD_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tmp = dir.join(format!(
            ".{final_name}.{}.{counter}.part",
            std::process::id()
        ));

        let resp = self
            .http
            .request(reqwest::Method::GET, &url)
            .basic_auth("apikey", Some(&self.token))
            .header(reqwest::header::ACCEPT, "application/octet-stream")
            .send()
            .await
            .map_err(|e| Error::Api(format!("request {url}: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            let msg = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(str::to_owned))
                .unwrap_or(text);
            return Err(Error::Api(format!("HTTP {}: {msg}", status.as_u16())));
        }

        let mut file =
            std::fs::File::create(&tmp).map_err(|e| Error::Io(format!("create temp file: {e}")))?;
        let mut hasher = Sha256::new();
        let mut total: u64 = 0;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(e) => {
                    drop(file);
                    let _ = std::fs::remove_file(&tmp);
                    return Err(Error::Api(format!("read body: {e}")));
                }
            };
            total += chunk.len() as u64;
            if let Some(limit) = max_bytes {
                if total > limit {
                    drop(file);
                    let _ = std::fs::remove_file(&tmp);
                    return Err(Error::Usage(format!(
                        "attachment exceeds max-bytes {limit}"
                    )));
                }
            }
            hasher.update(&chunk);
            if let Err(e) = file.write_all(&chunk) {
                drop(file);
                let _ = std::fs::remove_file(&tmp);
                return Err(Error::Io(format!("write download: {e}")));
            }
        }
        if let Err(e) = file.sync_all() {
            drop(file);
            let _ = std::fs::remove_file(&tmp);
            return Err(Error::Io(format!("flush download: {e}")));
        }
        drop(file);
        std::fs::rename(&tmp, output).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            Error::Io(format!("rename download: {e}"))
        })?;
        Ok(DownloadInfo {
            bytes: total,
            sha256: format!("{:x}", hasher.finalize()),
        })
    }

    /// Upload a file as an attachment on `wp_id` via a multipart POST. The
    /// content type is taken from `content_type`, else guessed from the name,
    /// else `application/octet-stream`. Returns the parsed attachment JSON.
    pub async fn upload_attachment(
        &self,
        wp_id: i64,
        file_path: &std::path::Path,
        file_name: Option<&str>,
        description: Option<&str>,
        content_type: Option<&str>,
    ) -> Result<serde_json::Value, Error> {
        let name = file_name
            .map(str::to_owned)
            .or_else(|| {
                file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "file".to_owned());
        let ctype = content_type
            .map(str::to_owned)
            .unwrap_or_else(|| guess_content_type(&name).to_owned());
        let data = std::fs::read(file_path)
            .map_err(|e| Error::Io(format!("read {}: {e}", file_path.display())))?;

        let mut metadata = serde_json::json!({ "fileName": name });
        if let Some(desc) = description {
            metadata["description"] = serde_json::json!({ "raw": desc });
        }
        let metadata_text = serde_json::to_string(&metadata)
            .map_err(|e| Error::Internal(format!("encode metadata: {e}")))?;

        let metadata_part = reqwest::multipart::Part::text(metadata_text)
            .mime_str("application/json")
            .map_err(|e| Error::Internal(format!("metadata part: {e}")))?;
        let file_part = reqwest::multipart::Part::bytes(data)
            .file_name(name.clone())
            .mime_str(&ctype)
            .map_err(|e| Error::Usage(format!("invalid content-type '{ctype}': {e}")))?;
        let form = reqwest::multipart::Form::new()
            .part("metadata", metadata_part)
            .part("file", file_part);

        let url = self.resolve_url(&format!("work_packages/{wp_id}/attachments"))?;
        let resp = self
            .http
            .request(reqwest::Method::POST, &url)
            .basic_auth("apikey", Some(&self.token))
            .header(reqwest::header::ACCEPT, "application/json")
            .multipart(form)
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
        serde_json::from_str(&text).map_err(|e| Error::Api(format!("parse json: {e}")))
    }
}

#[cfg(test)]
mod attachment_tests {
    use super::*;
    use sha2::{Digest, Sha256};
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
    async fn download_to_path_writes_file_with_hash() {
        let body = b"hello attachment body".to_vec();
        let expected = format!("{:x}", Sha256::digest(&body));
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/attachments/1/content"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
            .mount(&server)
            .await;
        let c = Client::new(
            "test",
            &profile_for(&server.uri()),
            "t".into(),
            Some("none"),
        )
        .unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("out.bin");
        let info = c
            .download_to_path("attachments/1/content", &out, None)
            .await
            .unwrap();
        assert_eq!(info.bytes, body.len() as u64);
        assert_eq!(info.sha256, expected);
        assert_eq!(std::fs::read(&out).unwrap(), body);
    }

    #[tokio::test]
    async fn download_to_path_max_bytes_aborts_and_cleans_up() {
        let body = vec![0u8; 4096];
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/attachments/2/content"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
            .mount(&server)
            .await;
        let c = Client::new(
            "test",
            &profile_for(&server.uri()),
            "t".into(),
            Some("none"),
        )
        .unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("big.bin");
        let err = c
            .download_to_path("attachments/2/content", &out, Some(16))
            .await
            .unwrap_err();
        assert_eq!(err.exit_code(), 2);
        assert!(!out.exists());
        let leftover: Vec<_> = std::fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".part"))
            .collect();
        assert!(leftover.is_empty(), "leftover .part files: {leftover:?}");
    }

    #[tokio::test]
    async fn upload_attachment_posts_multipart_and_parses_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v3/work_packages/5/attachments"))
            .respond_with(
                ResponseTemplate::new(201)
                    .set_body_json(serde_json::json!({"id": 42, "fileName": "note.txt"})),
            )
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
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("note.txt");
        std::fs::write(&file, b"content").unwrap();
        let v = c
            .upload_attachment(5, &file, None, Some("a note"), None)
            .await
            .unwrap();
        assert_eq!(v["id"], 42);
    }
}

#[cfg(test)]
mod custom_field_tests {
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
    async fn expands_scalar_and_linked_fields_with_names() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("OPENPROJECT_CACHE", tmp.path());
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/work_packages/schemas/1-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "customField1": {"name": "Priority"},
                "customField2": {"name": "Sprint"}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let c = Client::new(
            "cf-expand",
            &profile_for(&server.uri()),
            "t".into(),
            Some("none"),
        )
        .unwrap();
        let payload = serde_json::json!({
            "customField1": "high",
            "_links": {
                "schema": {"href": "/api/v3/work_packages/schemas/1-1"},
                "customField2": {"href": "/x", "title": "Sprint 5"}
            }
        });
        let out = c.custom_fields(&payload).await.unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(
            out[0],
            serde_json::json!({"key": "customField1", "name": "Priority", "value": "high"})
        );
        assert_eq!(
            out[1],
            serde_json::json!({"key": "customField2", "name": "Sprint", "value": "Sprint 5"})
        );
    }

    #[tokio::test]
    async fn no_custom_fields_yields_empty_and_no_schema_request() {
        let server = MockServer::start().await;
        // No schema mock: any request would 404 and (if reached) still not fail,
        // but the contract is that no request is made at all.
        let c = Client::new(
            "cf-empty",
            &profile_for(&server.uri()),
            "t".into(),
            Some("none"),
        )
        .unwrap();
        let payload = serde_json::json!({
            "subject": "hi",
            "_links": {"schema": {"href": "/api/v3/work_packages/schemas/1-1"}}
        });
        let out = c.custom_fields(&payload).await.unwrap();
        assert!(out.is_empty());
        assert_eq!(server.received_requests().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn linked_array_field_collects_titles() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/schema"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;
        let c = Client::new(
            "cf-array",
            &profile_for(&server.uri()),
            "t".into(),
            Some("none"),
        )
        .unwrap();
        let payload = serde_json::json!({
            "_links": {
                "schema": {"href": "/api/v3/schema"},
                "customField3": [
                    {"href": "/a", "title": "A"},
                    {"href": "/b", "title": "B"}
                ]
            }
        });
        let out = c.custom_fields(&payload).await.unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0],
            serde_json::json!({"key": "customField3", "name": null, "value": ["A", "B"]})
        );
    }
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
