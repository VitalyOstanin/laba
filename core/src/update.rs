//! Update changelog: GitHub release notes newer than the running version.
//!
//! The Tauri updater plugin decides *whether* an update exists and installs it,
//! but it only exposes the latest release's notes. To help the user understand
//! *why* to update, this collects the notes of every published release strictly
//! newer than the running version — a cumulative changelog from the user's
//! version up to the latest — via the public GitHub releases API. Display only;
//! it never downloads or applies anything.

use serde::{Deserialize, Serialize};

use crate::error::Error;

/// Repository whose releases back the changelog and updater endpoint.
pub const UPDATE_OWNER: &str = "VitalyOstanin";
pub const UPDATE_REPO: &str = "laboro";

const GITHUB_API: &str = "https://api.github.com";

/// One release presented in the changelog, newest first. `version` is
/// normalized (no leading `v`); `body` is the release notes markdown.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseNote {
    pub version: String,
    pub name: Option<String>,
    pub body: String,
    pub published_at: Option<String>,
}

/// The subset of the GitHub release object this module reads.
#[derive(Debug, Clone, Deserialize)]
struct GhRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    published_at: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// Tag without a leading `v` (`v1.2.3` -> `1.2.3`), for semver parsing.
fn strip_v(tag: &str) -> &str {
    tag.strip_prefix('v')
        .or_else(|| tag.strip_prefix('V'))
        .unwrap_or(tag)
}

/// Keep releases strictly newer than `current`, drop drafts/prereleases and
/// tags that are not valid semver, and sort newest first. Pure, so the
/// filtering and ordering are unit-tested without the network. A `current` that
/// is not valid semver keeps every published release (best-effort: show all
/// available notes rather than nothing).
fn newer_release_notes(current: &str, releases: Vec<GhRelease>) -> Vec<ReleaseNote> {
    let cur = semver::Version::parse(strip_v(current)).ok();
    let mut kept: Vec<(semver::Version, ReleaseNote)> = releases
        .into_iter()
        .filter(|r| !r.draft && !r.prerelease)
        .filter_map(|r| {
            let v = semver::Version::parse(strip_v(&r.tag_name)).ok()?;
            if let Some(c) = &cur {
                if v <= *c {
                    return None;
                }
            }
            let note = ReleaseNote {
                version: v.to_string(),
                name: r.name,
                body: r.body.unwrap_or_default(),
                published_at: r.published_at,
            };
            Some((v, note))
        })
        .collect();
    // Newest first.
    kept.sort_by(|a, b| b.0.cmp(&a.0));
    kept.into_iter().map(|(_, n)| n).collect()
}

/// Fetch the cumulative changelog for versions newer than `current` from the
/// public GitHub releases API. Returns an empty list when nothing is newer.
pub async fn changelog_since(current: &str) -> Result<Vec<ReleaseNote>, Error> {
    changelog_from(GITHUB_API, current).await
}

/// [`changelog_since`] against an arbitrary API base, so tests can point it at a
/// local mock server.
async fn changelog_from(api_base: &str, current: &str) -> Result<Vec<ReleaseNote>, Error> {
    let url = format!("{api_base}/repos/{UPDATE_OWNER}/{UPDATE_REPO}/releases?per_page=100");
    let client = reqwest::Client::builder()
        // GitHub rejects requests without a User-Agent.
        .user_agent(concat!("laboro/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| Error::Io(format!("build http client: {e}")))?;
    let resp = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| Error::Io(format!("fetch releases: {e}")))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(Error::Api(format!("GitHub releases: HTTP {status}")));
    }
    let releases: Vec<GhRelease> = resp
        .json()
        .await
        .map_err(|e| Error::Api(format!("parse releases: {e}")))?;
    Ok(newer_release_notes(current, releases))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn rel(tag: &str) -> GhRelease {
        GhRelease {
            tag_name: tag.into(),
            name: Some(format!("Release {tag}")),
            body: Some(format!("notes for {tag}")),
            published_at: None,
            draft: false,
            prerelease: false,
        }
    }

    #[test]
    fn keeps_only_newer_sorted_desc_and_strips_v() {
        let releases = vec![rel("v0.1.0"), rel("v0.3.0"), rel("0.2.0"), rel("v0.1.0")];
        let notes = newer_release_notes("0.1.0", releases);
        let versions: Vec<&str> = notes.iter().map(|n| n.version.as_str()).collect();
        assert_eq!(versions, ["0.3.0", "0.2.0"]);
        // Body carried through, version normalized without the leading v.
        assert_eq!(notes[0].body, "notes for v0.3.0");
    }

    #[test]
    fn drops_drafts_prereleases_and_unparsable_tags() {
        let mut draft = rel("v0.4.0");
        draft.draft = true;
        let mut pre = rel("v0.5.0");
        pre.prerelease = true;
        let releases = vec![draft, pre, rel("nightly"), rel("v0.2.0")];
        let notes = newer_release_notes("0.1.0", releases);
        let versions: Vec<&str> = notes.iter().map(|n| n.version.as_str()).collect();
        assert_eq!(versions, ["0.2.0"]);
    }

    #[test]
    fn unparsable_current_keeps_all_published() {
        let releases = vec![rel("v0.1.0"), rel("v0.2.0")];
        let notes = newer_release_notes("not-a-version", releases);
        assert_eq!(notes.len(), 2);
    }

    #[tokio::test]
    async fn changelog_from_fetches_and_filters() {
        let server = MockServer::start().await;
        let body = serde_json::json!([
            { "tag_name": "v0.3.0", "name": "0.3.0", "body": "three" },
            { "tag_name": "v0.2.0", "name": "0.2.0", "body": "two" },
            { "tag_name": "v0.1.0", "name": "0.1.0", "body": "one" },
        ]);
        Mock::given(method("GET"))
            .and(path(format!(
                "/repos/{UPDATE_OWNER}/{UPDATE_REPO}/releases"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;
        let notes = changelog_from(&server.uri(), "0.1.0").await.unwrap();
        let versions: Vec<&str> = notes.iter().map(|n| n.version.as_str()).collect();
        assert_eq!(versions, ["0.3.0", "0.2.0"]);
    }

    #[tokio::test]
    async fn changelog_from_errors_on_http_failure() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let err = changelog_from(&server.uri(), "0.1.0").await.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
    }
}
