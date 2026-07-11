pub mod api;
pub mod attachment;
pub mod auth;
pub mod cache;
pub mod comment;
pub mod notification;
pub mod relation;
pub mod server;
pub mod time;
pub mod wp;

use std::path::PathBuf;

use taskstream_core::client::Client;
use taskstream_core::config::{default_config_path, Backend, Config, ServerProfile};
use taskstream_core::error::Error;
use taskstream_core::secrets::Secrets;

use crate::cli::Globals;

/// Resolve the effective config path from the global `--config` flag.
pub fn config_path(flag: &Option<PathBuf>) -> PathBuf {
    flag.clone().unwrap_or_else(default_config_path)
}

/// Resolve the active server and return its name and a clone of its profile,
/// without requiring a token. Commands branch on `profile.backend` before
/// building a backend-specific client.
pub fn load_profile(g: &Globals) -> Result<(String, ServerProfile), Error> {
    let path = config_path(&g.config);
    let cfg = Config::load(&path)?;
    let name = cfg.resolve_server_name(g.server.as_deref())?;
    let profile = cfg.servers[&name].clone();
    Ok((name, profile))
}

/// For github-backed commands, verify the `gh` CLI is installed and signed in,
/// returning a friendly, actionable error otherwise. A no-op for OpenProject.
/// The update checker does not use `gh`; only the GitHub task backend does.
pub fn ensure_gh_ready(profile: &ServerProfile) -> Result<(), Error> {
    use taskstream_core::github::{gh_status_for_host, GhStatus};
    if profile.backend != Backend::Github {
        return Ok(());
    }
    match gh_status_for_host(&profile.base_url) {
        GhStatus::Ready => Ok(()),
        GhStatus::Missing => Err(Error::Usage(
            "the github backend needs the GitHub CLI (gh), which is not installed. \
             Install it: https://github.com/cli/cli#installation"
                .into(),
        )),
        GhStatus::Unauthenticated => Err(Error::Usage(
            "the github backend needs the GitHub CLI signed in. \
             Run 'gh auth login', then retry."
                .into(),
        )),
    }
}

/// Reject a command the active backend cannot serve. `what` names the command
/// for the error; the message also lists what the github backend does support.
pub fn require_openproject(profile: &ServerProfile, what: &str) -> Result<(), Error> {
    match profile.backend {
        Backend::OpenProject => Ok(()),
        Backend::Github => Err(Error::Usage(format!(
            "{what} is not supported by the github backend \
             (read-only: 'wp list', 'notification list')"
        ))),
    }
}

/// Build a [`Client`] from the global flags: resolve the active server, load its
/// profile, and pick the token (`--token`/env, then the stored secret). Returns
/// the resolved server name alongside the client. Rejects non-OpenProject
/// backends so their commands fail with a clear message rather than a confusing
/// "no token" error.
pub fn build_client(g: &Globals) -> Result<(String, Client), Error> {
    // Load the config directly (not via `load_profile`) so the global proxy
    // default is available alongside the profile without a second read.
    let cfg = Config::load(&config_path(&g.config))?;
    let name = cfg.resolve_server_name(g.server.as_deref())?;
    let profile = cfg.servers[&name].clone();
    require_openproject(&profile, "this command")?;
    let secrets = Secrets::new(Secrets::default_fallback_path());
    let token = g
        .token
        .clone()
        .or(secrets.get(&name)?)
        .ok_or_else(|| Error::Auth(format!("no token for '{name}'")))?;
    let client = Client::new_with_global(
        &name,
        &profile,
        token,
        g.proxy.as_deref(),
        cfg.proxy.as_deref(),
    )?;
    Ok((name, client))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(backend: Backend) -> ServerProfile {
        ServerProfile {
            base_url: "github.com".into(),
            backend,
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

    #[test]
    fn require_openproject_allows_openproject() {
        assert!(require_openproject(&profile(Backend::OpenProject), "x").is_ok());
    }

    #[test]
    fn require_openproject_rejects_github_as_usage_error() {
        let err = require_openproject(&profile(Backend::Github), "'time log'").unwrap_err();
        assert_eq!(err.exit_code(), 2);
        assert!(err.to_string().contains("github backend"));
    }
}
