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
use taskstream_core::config::{default_config_path, Config};
use taskstream_core::error::Error;
use taskstream_core::secrets::Secrets;

use crate::cli::Globals;

/// Resolve the effective config path from the global `--config` flag.
pub fn config_path(flag: &Option<PathBuf>) -> PathBuf {
    flag.clone().unwrap_or_else(default_config_path)
}

/// Build a [`Client`] from the global flags: resolve the active server, load its
/// profile, and pick the token (`--token`/env, then the stored secret). Returns
/// the resolved server name alongside the client.
pub fn build_client(g: &Globals) -> Result<(String, Client), Error> {
    let path = config_path(&g.config);
    let cfg = Config::load(&path)?;
    let name = cfg.resolve_server_name(g.server.as_deref())?;
    let profile = &cfg.servers[&name];
    let secrets = Secrets::new(Secrets::default_fallback_path());
    let token = g
        .token
        .clone()
        .or(secrets.get(&name)?)
        .ok_or_else(|| Error::Auth(format!("no token for '{name}'")))?;
    let client = Client::new(&name, profile, token, g.proxy.as_deref())?;
    Ok((name, client))
}
