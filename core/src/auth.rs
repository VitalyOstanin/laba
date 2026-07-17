//! Shared OpenProject login: validate a token against `users/me`, reject a
//! duplicate account, and store the token. Used by both the CLI (`auth login`)
//! and the GUI (`login_server` command) so the validation and duplicate rules
//! live in one place.

use serde_json::Value;

use crate::client::Client;
use crate::config::{BackendKind, Config, ServerProfile};
use crate::error::Error;
use crate::secrets::Secrets;

/// Stable account identity from a `users/me` payload: the login if present,
/// otherwise the numeric id rendered as a string.
pub fn identity(me: &Value) -> Option<String> {
    me.get("login")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .or_else(|| me.get("id").map(|v| v.to_string()))
}

/// The name of an existing server on the same base URL authenticated as the same
/// account `id`, if any. Profiles whose identity cannot be confirmed (missing or
/// expired token, unreachable host) cannot prove a duplicate and are skipped.
async fn duplicate_server(
    cfg: &Config,
    secrets: &Secrets,
    name: &str,
    profile: &ServerProfile,
    id: &str,
) -> Result<Option<String>, Error> {
    for (other, op) in &cfg.servers {
        if other == name || op.base_url != profile.base_url {
            continue;
        }
        let Some(other_token) = secrets.get(other)? else {
            continue;
        };
        let oc = Client::new(other, op, other_token, None)?;
        if let Ok(other_me) = oc.get_json_retrying("users/me", 1).await {
            if identity(&other_me).as_deref() == Some(id) {
                return Ok(Some(other.clone()));
            }
        }
    }
    Ok(None)
}

/// Validate `token` for the server `name` against `users/me` and store it,
/// rejecting a duplicate (another profile with the same base URL and account)
/// unless `force`. The GitHub backend authenticates via `gh` and is rejected.
pub async fn login_and_store(
    cfg: &Config,
    secrets: &Secrets,
    name: &str,
    token: &str,
    force: bool,
) -> Result<(), Error> {
    let profile = cfg
        .servers
        .get(name)
        .ok_or_else(|| Error::Usage(format!("unknown server '{name}'")))?;
    if profile.backend == BackendKind::Github {
        return Err(Error::Usage(
            "the github backend authenticates via gh; run 'gh auth login' instead".into(),
        ));
    }
    let token = token.trim();
    if token.is_empty() {
        return Err(Error::Usage("empty token".into()));
    }
    // Validate the token and read the account identity, needed to detect a
    // duplicate (same base URL + same user).
    let client = Client::new(name, profile, token.to_owned(), None)?;
    let me = client.get_json_retrying("users/me", 3).await?;
    if !force {
        if let Some(id) = identity(&me) {
            if let Some(other) = duplicate_server(cfg, secrets, name, profile, &id).await? {
                return Err(Error::Usage(format!(
                    "user '{id}' at {} is already authenticated as server '{other}'; use --force to add anyway",
                    profile.base_url
                )));
            }
        }
    }
    secrets.set(name, token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn identity_prefers_login_over_id() {
        assert_eq!(
            identity(&json!({"login": "jdoe", "id": 7})).as_deref(),
            Some("jdoe")
        );
    }

    #[test]
    fn identity_falls_back_to_id() {
        assert_eq!(identity(&json!({"id": 7})).as_deref(), Some("7"));
    }

    #[test]
    fn identity_none_when_absent() {
        assert_eq!(identity(&json!({"name": "no id here"})), None);
    }
}
