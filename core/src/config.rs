use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerProfile {
    pub base_url: String,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default = "default_true")]
    pub verify_ssl: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
}

fn default_timeout() -> u64 {
    30
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_server: Option<String>,
    #[serde(default)]
    pub servers: BTreeMap<String, ServerProfile>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Config, Error> {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|e| Error::Config(format!("parse {}: {e}", path.display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(Error::Io(format!("read {}: {e}", path.display()))),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), Error> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| Error::Io(format!("mkdir {}: {e}", dir.display())))?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| Error::Internal(format!("serialize config: {e}")))?;
        std::fs::write(path, text).map_err(|e| Error::Io(format!("write {}: {e}", path.display())))
    }

    /// Resolve the active server name: explicit `--server` wins, then
    /// `OPENPROJECT_SERVER`, then `default_server`.
    pub fn resolve_server_name(&self, flag: Option<&str>) -> Result<String, Error> {
        let from_env = std::env::var("OPENPROJECT_SERVER").ok();
        let name = flag
            .map(str::to_owned)
            .or(from_env)
            .or_else(|| self.default_server.clone())
            .ok_or_else(|| {
                Error::Usage(
                    "no server selected: pass --server or set a default (server set-default)"
                        .into(),
                )
            })?;
        if !self.servers.contains_key(&name) {
            return Err(Error::Usage(format!("unknown server '{name}'")));
        }
        Ok(name)
    }
}

/// Default config path: `$XDG_CONFIG_HOME/taskstream/config.json`.
pub fn default_config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"));
    base.join("taskstream").join("config.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_missing_returns_default() {
        let p = std::path::Path::new("/nonexistent/taskstream/config.json");
        assert_eq!(Config::load(p).unwrap(), Config::default());
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn save_then_load_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut cfg = Config::default();
        cfg.default_server = Some("primary".into());
        cfg.servers.insert(
            "primary".into(),
            ServerProfile {
                base_url: "https://host.example/openproject".into(),
                timeout: 30,
                verify_ssl: true,
                proxy: Some("socks5://127.0.0.1:10808".into()),
            },
        );
        cfg.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap(), cfg);
    }

    #[allow(clippy::field_reassign_with_default)]
    fn cfg_with(names: &[&str], default: Option<&str>) -> Config {
        let mut c = Config::default();
        c.default_server = default.map(str::to_owned);
        for n in names {
            c.servers.insert(
                (*n).into(),
                ServerProfile {
                    base_url: "u".into(),
                    timeout: 30,
                    verify_ssl: true,
                    proxy: None,
                },
            );
        }
        c
    }

    #[test]
    fn flag_beats_default() {
        let c = cfg_with(&["a", "b"], Some("a"));
        assert_eq!(c.resolve_server_name(Some("b")).unwrap(), "b");
    }

    #[test]
    fn falls_back_to_default() {
        let c = cfg_with(&["a"], Some("a"));
        assert_eq!(c.resolve_server_name(None).unwrap(), "a");
    }

    #[test]
    fn unknown_server_is_usage_error() {
        let c = cfg_with(&["a"], None);
        assert_eq!(c.resolve_server_name(Some("x")).unwrap_err().exit_code(), 2);
    }
}
