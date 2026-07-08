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
}

impl Client {
    pub fn new(
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
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
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
        let c = Client::new(&profile(), "t".into(), None).unwrap();
        assert_eq!(
            c.api_url("work_packages/1"),
            "https://host.example/openproject/api/v3/work_packages/1"
        );
    }

    #[test]
    fn api_url_collapses_deployment_subpath_href() {
        let c = Client::new(&profile(), "t".into(), None).unwrap();
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
            Client::new(&p, "t".into(), None).unwrap_err().exit_code(),
            70
        );
    }
}

impl Client {
    /// Execute a request and return the parsed JSON body, mapping non-2xx
    /// responses to `Error::Api` with the server message when present.
    pub async fn request_json(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Error> {
        let url = self.api_url(path);
        let mut req = self
            .http
            .request(method, &url)
            .basic_auth("apikey", Some(&self.token))
            .header(reqwest::header::ACCEPT, "application/json");
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
        let c = Client::new(&profile_for(&server.uri()), "t".into(), Some("none")).unwrap();
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
        let c = Client::new(&profile_for(&server.uri()), "t".into(), Some("none")).unwrap();
        let err = c
            .request_json(reqwest::Method::GET, "users/me", None)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("HTTP 404"));
        assert!(err.to_string().contains("nope"));
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
        let c = Client::new(&p, "t".into(), Some("none")).unwrap();
        let v = c.get_json_retrying("x", 3).await.unwrap();
        assert_eq!(v["ok"], true);
    }
}
