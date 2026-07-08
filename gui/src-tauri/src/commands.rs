//! Thin Tauri commands wrapping `taskstream_core`. Business logic stays in core.

use serde::Serialize;
use serde_json::Value;
use taskstream_core::backend;
use taskstream_core::client::Client;
use taskstream_core::config::{default_config_path, Backend, Config};
use taskstream_core::resources::time;
use taskstream_core::secrets::Secrets;
use taskstream_core::settings::{default_settings_path, Settings, TimelogStart};
use taskstream_core::timelog::{self, DayCell, TimelogStatus};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ServerInfo {
    pub name: String,
    pub base_url: String,
    pub backend: String, // "openproject" | "github"
    pub is_default: bool,
    pub poll_secs: u64,
    /// Not temporarily disabled in the GUI.
    pub enabled: bool,
}

fn backend_str(b: Backend) -> &'static str {
    match b {
        Backend::OpenProject => "openproject",
        Backend::Github => "github",
    }
}

/// Build the server list for the UI switcher (pure, testable). The effective
/// poll interval reflects any per-server override in settings.
pub fn server_infos(cfg: &Config, settings: &Settings) -> Vec<ServerInfo> {
    cfg.servers
        .iter()
        .map(|(name, p)| ServerInfo {
            name: name.clone(),
            base_url: p.base_url.clone(),
            backend: backend_str(p.backend).into(),
            is_default: cfg.default_server.as_deref() == Some(name.as_str()),
            poll_secs: settings.effective_poll_secs(name, p.backend),
            enabled: settings.is_enabled(name),
        })
        .collect()
}

fn load_cfg() -> Result<Config, String> {
    Config::load(&default_config_path()).map_err(|e| e.to_string())
}

fn load_settings() -> Result<Settings, String> {
    Settings::load(&default_settings_path()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_servers() -> Result<Vec<ServerInfo>, String> {
    Ok(server_infos(&load_cfg()?, &load_settings()?))
}

#[tauri::command]
pub fn get_settings() -> Result<Settings, String> {
    load_settings()
}

#[tauri::command]
pub fn save_settings(settings: Settings) -> Result<(), String> {
    settings
        .save(&default_settings_path())
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelogResult {
    /// Whether at least one timelog-capable server has a valid start date. When
    /// false the rest is empty.
    pub configured: bool,
    pub status: TimelogStatus,
    pub timeline: Vec<DayCell>,
    /// Aggregate window start = earliest per-server start (`YYYY-MM-DD`, empty
    /// when unconfigured).
    pub start: String,
    /// True while any timelog-capable server's start is still the auto-seeded
    /// first-launch date, so the UI prompts the user to set real start dates.
    pub start_is_default: bool,
    /// Servers excluded from timelog because their backend has no time tracking
    /// (requirement 22), for a UI hint.
    pub excluded: Vec<String>,
}

/// Aggregate my logged time across timelog-capable servers and compute the
/// work-log status and timeline. Each server contributes entries from its own
/// start date; the aggregate plan runs from the earliest start (requirement 21).
/// Servers whose backend has no time tracking (GitHub) are excluded
/// (requirement 22). A server first seen here is seeded with today's date as an
/// auto start so timelog has a sensible window until the user sets a real one.
/// Per-server fetch failures are skipped so one bad server does not blank the
/// whole indicator.
#[tauri::command]
pub async fn get_timelog() -> Result<TimelogResult, String> {
    let cfg = load_cfg()?;
    let mut settings = load_settings()?;
    let today = timelog::fmt(timelog::today_local());

    let mut entries: Vec<(String, i64)> = Vec::new();
    let mut excluded: Vec<String> = Vec::new();
    let mut starts: Vec<String> = Vec::new();
    let mut any_auto = false;
    let mut dirty = false;

    for (name, p) in &cfg.servers {
        // Temporarily disabled servers are skipped entirely.
        if settings.disabled_servers.contains(name) {
            continue;
        }
        if !p.backend.supports_timelog() {
            excluded.push(name.clone());
            continue;
        }
        // Seed a start date the first time we see this server.
        let entry = settings
            .timelog_start
            .entry(name.clone())
            .or_insert_with(|| {
                dirty = true;
                TimelogStart {
                    date: today.clone(),
                    auto: true,
                }
            });
        let since = entry.date.clone();
        if entry.auto {
            any_auto = true;
        }
        if timelog::parse_date(&since).is_some() {
            starts.push(since.clone());
        }

        let token = match token_for(name, p.backend)? {
            Some(t) => t,
            None => continue,
        };
        let client = match Client::new("", p, token, None) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let list = time::list(
            &client,
            Some("me"),
            None,
            None,
            Some(&since),
            Some(&today),
            1,
            None,
            false,
        )
        .await;
        let Ok(Value::Array(arr)) = list else {
            continue;
        };
        for te in arr {
            let day = te.get("spentOn").and_then(|v| v.as_str()).unwrap_or("");
            let hours = te.get("hours").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if !day.is_empty() {
                entries.push((day.to_string(), timelog::minutes_from_hours(hours)));
            }
        }
    }

    if dirty {
        // Best-effort: persist the seeded starts; ignore write failures.
        let _ = settings.save(&default_settings_path());
    }

    // ISO dates sort lexicographically, so the min string is the earliest start.
    let earliest = starts.iter().min().cloned();
    match earliest.and_then(|s| timelog::compute(&entries, &s).map(|r| (s, r))) {
        Some((start, (status, timeline))) => Ok(TimelogResult {
            configured: true,
            status,
            timeline,
            start,
            start_is_default: any_auto,
            excluded,
        }),
        None => Ok(TimelogResult {
            configured: false,
            status: timelog::empty_status(),
            timeline: Vec::new(),
            start: String::new(),
            start_is_default: any_auto,
            excluded,
        }),
    }
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
        let infos = server_infos(&cfg(), &Settings::default());
        assert_eq!(infos.len(), 2);
        let work = infos.iter().find(|i| i.name == "work").unwrap();
        assert_eq!(work.backend, "openproject");
        assert!(work.is_default);
        assert_eq!(work.poll_secs, 120);
        assert!(work.enabled);
        let gh = infos.iter().find(|i| i.name == "gh").unwrap();
        assert_eq!(gh.backend, "github");
        assert!(!gh.is_default);
        assert_eq!(gh.poll_secs, 900);
    }

    #[test]
    fn server_infos_apply_poll_override() {
        let mut settings = Settings::default();
        settings.poll_override.insert("work".into(), 300);
        let infos = server_infos(&cfg(), &settings);
        let work = infos.iter().find(|i| i.name == "work").unwrap();
        assert_eq!(work.poll_secs, 300);
        // Non-overridden server keeps its backend default.
        let gh = infos.iter().find(|i| i.name == "gh").unwrap();
        assert_eq!(gh.poll_secs, 900);
    }

    #[test]
    fn server_infos_mark_disabled() {
        let mut settings = Settings::default();
        settings.disabled_servers.insert("gh".into());
        let infos = server_infos(&cfg(), &settings);
        assert!(infos.iter().find(|i| i.name == "work").unwrap().enabled);
        assert!(!infos.iter().find(|i| i.name == "gh").unwrap().enabled);
    }
}
