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
    let (name, profile) = load_profile(g)?;
    require_openproject(&profile, "this command")?;
    let secrets = Secrets::new(Secrets::default_fallback_path());
    let token = g
        .token
        .clone()
        .or(secrets.get(&name)?)
        .ok_or_else(|| Error::Auth(format!("no token for '{name}'")))?;
    let client = Client::new(&name, &profile, token, g.proxy.as_deref())?;
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
