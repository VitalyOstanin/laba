//! Thin Tauri commands wrapping `taskstream_core`. Business logic stays in core.

use serde::Serialize;
use serde_json::Value;
use taskstream_core::backend;
use taskstream_core::config::{default_config_path, Backend, Config};
use taskstream_core::secrets::Secrets;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ServerInfo {
    pub name: String,
    pub base_url: String,
    pub backend: String, // "openproject" | "github"
    pub is_default: bool,
    pub poll_secs: u64,
}

fn backend_str(b: Backend) -> &'static str {
    match b {
        Backend::OpenProject => "openproject",
        Backend::Github => "github",
    }
}

/// Build the server list for the UI switcher (pure, testable).
pub fn server_infos(cfg: &Config) -> Vec<ServerInfo> {
    cfg.servers
        .iter()
        .map(|(name, p)| ServerInfo {
            name: name.clone(),
            base_url: p.base_url.clone(),
            backend: backend_str(p.backend).into(),
            is_default: cfg.default_server.as_deref() == Some(name.as_str()),
            poll_secs: p.backend.default_poll_secs(),
        })
        .collect()
}

fn load_cfg() -> Result<Config, String> {
    Config::load(&default_config_path()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_servers() -> Result<Vec<ServerInfo>, String> {
    Ok(server_infos(&load_cfg()?))
}

#[tauri::command]
pub async fn list_tasks(server: String) -> Result<Vec<Value>, String> {
    let cfg = load_cfg()?;
    let profile = cfg
        .servers
        .get(&server)
        .ok_or_else(|| format!("unknown server '{server}'"))?
        .clone();
    let token = token_for(&server, profile.backend)?;
    backend::list_tasks(&profile, token.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_notifications(server: String) -> Result<Vec<Value>, String> {
    let cfg = load_cfg()?;
    let profile = cfg
        .servers
        .get(&server)
        .ok_or_else(|| format!("unknown server '{server}'"))?
        .clone();
    let token = token_for(&server, profile.backend)?;
    backend::list_notifications(&profile, token.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// OpenProject servers need a token from the keyring/file secret store; GitHub
/// uses `gh` and needs none.
fn token_for(server: &str, backend: Backend) -> Result<Option<String>, String> {
    match backend {
        Backend::Github => Ok(None),
        Backend::OpenProject => {
            let secrets = Secrets::new(Secrets::default_fallback_path());
            secrets.get(server).map_err(|e| e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use taskstream_core::config::ServerProfile;

    fn cfg() -> Config {
        let mut servers = BTreeMap::new();
        servers.insert(
            "work".into(),
            ServerProfile {
                base_url: "https://op.example".into(),
                backend: Backend::OpenProject,
                timeout: 30,
                verify_ssl: true,
                proxy: None,
            },
        );
        servers.insert(
            "gh".into(),
            ServerProfile {
                base_url: "github.com".into(),
                backend: Backend::Github,
                timeout: 30,
                verify_ssl: true,
                proxy: None,
            },
        );
        Config {
            default_server: Some("work".into()),
            servers,
        }
    }

    #[test]
    fn server_infos_lists_all_with_backend_and_default() {
        let infos = server_infos(&cfg());
        assert_eq!(infos.len(), 2);
        let work = infos.iter().find(|i| i.name == "work").unwrap();
        assert_eq!(work.backend, "openproject");
        assert!(work.is_default);
        assert_eq!(work.poll_secs, 120);
        let gh = infos.iter().find(|i| i.name == "gh").unwrap();
        assert_eq!(gh.backend, "github");
        assert!(!gh.is_default);
        assert_eq!(gh.poll_secs, 900);
    }
}
