//! Attachment resource operations (`attachment` command family): list, get,
//! download, upload, delete. Ported 1:1 from the predecessor Python
//! `attachment.py`.

use std::path::Path;

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

/// List the attachments of a work package.
pub async fn list(
    client: &Client,
    work_package: i64,
    offset: i64,
    limit: Option<i64>,
    raw: bool,
) -> Result<Value, Error> {
    let q = paging_query(offset, limit);
    let payload = client
        .request_json_query(
            reqwest::Method::GET,
            &format!("work_packages/{work_package}/attachments"),
            &q,
            None,
        )
        .await?;
    let elements = normalize::collection(&payload);
    if raw {
        return Ok(Value::Array(elements));
    }
    let out = elements.iter().map(normalize::attachment).collect();
    Ok(Value::Array(out))
}

/// Fetch a single attachment by id.
pub async fn get(client: &Client, id: i64, raw: bool) -> Result<Value, Error> {
    let payload = client
        .request_json(reqwest::Method::GET, &format!("attachments/{id}"), None)
        .await?;
    if raw {
        Ok(payload)
    } else {
        Ok(normalize::attachment(&payload))
    }
}

/// Download an attachment's content to a file.
pub async fn download(
    client: &Client,
    id: i64,
    output: &Path,
    max_bytes: Option<u64>,
) -> Result<Value, Error> {
    let info = client
        .download_to_path(&format!("attachments/{id}/content"), output, max_bytes)
        .await?;
    Ok(json!({
        "downloaded": id,
        "path": output.to_string_lossy(),
        "bytes": info.bytes,
        "sha256": info.sha256,
    }))
}

/// Upload a file as an attachment on a work package.
pub async fn upload(
    client: &Client,
    work_package: i64,
    file: &Path,
    name: Option<&str>,
    description: Option<&str>,
    content_type: Option<&str>,
    raw: bool,
) -> Result<Value, Error> {
    let payload = client
        .upload_attachment(work_package, file, name, description, content_type)
        .await?;
    if raw {
        Ok(payload)
    } else {
        Ok(normalize::attachment(&payload))
    }
}

/// Delete an attachment by id.
pub async fn delete(client: &Client, id: i64) -> Result<Value, Error> {
    client.delete(&format!("attachments/{id}")).await?;
    Ok(json!({"deleted": id}))
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
        }
    }

    fn client_for(server: &MockServer, name: &str) -> Client {
        Client::new(name, &profile_for(&server.uri()), "t".into(), Some("none")).unwrap()
    }

    fn attachment_element(id: i64, file_name: &str) -> Value {
        json!({
            "id": id,
            "fileName": file_name,
            "fileSize": 42,
            "contentType": "text/plain",
            "description": {"raw": "note"},
            "createdAt": "2026-01-01T00:00:00Z",
            "_links": {
                "author": {"href": "/api/v3/users/8", "title": "U"},
                "downloadLocation": {"href": "/api/v3/attachments/3/content"}
            }
        })
    }

    #[tokio::test]
    async fn list_normalizes_attachments() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/work_packages/5/attachments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_embedded": {"elements": [attachment_element(3, "a.txt")]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "attachment-list");
        let out = list(&c, 5, 1, None, false).await.unwrap();
        let arr = out.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], json!(3));
        assert_eq!(arr[0]["fileName"], json!("a.txt"));
        assert_eq!(
            arr[0]["downloadUrl"],
            json!("/api/v3/attachments/3/content")
        );
    }

    #[tokio::test]
    async fn get_normalizes_attachment() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/attachments/3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(attachment_element(3, "a.txt")))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "attachment-get");
        let out = get(&c, 3, false).await.unwrap();
        assert_eq!(out["id"], json!(3));
        assert_eq!(out["fileName"], json!("a.txt"));
    }

    #[tokio::test]
    async fn download_writes_file_and_returns_info() {
        let tmp = tempfile::tempdir().unwrap();
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/attachments/3/content"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello".to_vec()))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "attachment-download");
        let out_path = tmp.path().join("a.txt");
        let out = download(&c, 3, &out_path, None).await.unwrap();
        assert_eq!(out["downloaded"], json!(3));
        assert_eq!(out["bytes"], json!(5));
        assert!(out["sha256"].is_string());
        assert_eq!(out["path"], json!(out_path.to_string_lossy()));
        assert_eq!(std::fs::read(&out_path).unwrap(), b"hello");
    }

    #[tokio::test]
    async fn upload_normalizes_attachment() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("up.txt");
        std::fs::write(&file, b"data").unwrap();
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v3/work_packages/5/attachments"))
            .respond_with(ResponseTemplate::new(201).set_body_json(attachment_element(9, "up.txt")))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "attachment-upload");
        let out = upload(&c, 5, &file, None, None, None, false).await.unwrap();
        assert_eq!(out["id"], json!(9));
        assert_eq!(out["fileName"], json!("up.txt"));
    }

    #[tokio::test]
    async fn delete_sends_request_and_returns_id() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v3/attachments/3"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;
        let c = client_for(&server, "attachment-delete");
        let out = delete(&c, 3).await.unwrap();
        assert_eq!(out, json!({"deleted": 3}));
    }
}
