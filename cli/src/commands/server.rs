use std::path::PathBuf;

use clap::Subcommand;
use laba_core::config::{Backend, Config, ServerProfile, StatusColor};
use laba_core::error::Error;
use laba_core::secrets::Secrets;

#[derive(Debug, Subcommand)]
pub enum ServerCmd {
    /// List server profiles.
    List,
    /// Add a server profile. `name` is the short name / identifier (the key used
    /// in the keyring and everywhere a server is referenced).
    Add {
        name: String,
        #[arg(long)]
        url: String,
        /// Backend: `openproject` (default) or `github`. GitHub servers use the
        /// `gh` CLI and need no token; `url` is the GitHub host (e.g. github.com).
        #[arg(long, default_value = "openproject")]
        backend: String,
        /// Full display name (shown in tooltips / settings). Defaults to `name`.
        #[arg(long)]
        display_name: Option<String>,
        #[arg(long)]
        proxy: Option<String>,
        #[arg(long, default_value_t = 30)]
        timeout: u64,
        #[arg(long, default_value_t = true)]
        verify_ssl: bool,
        /// Poll interval in seconds. Omitted falls back to the backend default.
        #[arg(long)]
        poll_secs: Option<u64>,
        /// Add the server disabled in the GUI (not polled/shown). Enabled by default.
        #[arg(long)]
        disabled: bool,
        /// Mark this profile as the default.
        #[arg(long)]
        default: bool,
        /// Replace an existing profile with the same name.
        #[arg(long)]
        force: bool,
    },
    /// Remove a server profile (and its stored token).
    Remove { name: String },
    /// Rename a server: re-key the profile, update the default, and move its
    /// stored token to the new key.
    Rename { old: String, new: String },
    /// Set the default server.
    SetDefault { name: String },
    /// Set or clear the row tint for a workflow status on a server. The status
    /// string must match exactly what the server reports.
    StatusColor {
        /// Server short name (defaults to the default server).
        #[arg(long)]
        server: Option<String>,
        /// Exact status string as it appears on the server.
        status: String,
        /// Tint token: danger | warn | success | dimmed. Omit with --clear.
        color: Option<String>,
        /// Remove the mapping for `status` instead of setting it.
        #[arg(long)]
        clear: bool,
    },
    /// Set, clear, or show a server's proxy override. A URL routes through that
    /// proxy; `direct` forces a direct connection; empty or --clear inherits the
    /// global default, then the ambient env. With no value, prints the current
    /// override.
    Proxy {
        /// Server short name (defaults to the default server).
        #[arg(long)]
        server: Option<String>,
        /// Proxy URL, `direct`, or empty to clear. Omit to print the current value.
        proxy: Option<String>,
        /// Clear the override (inherit the global default / env).
        #[arg(long)]
        clear: bool,
    },
    /// Set, clear, or show the global default proxy (used by servers without
    /// their own override). A URL or `direct`; empty or --clear clears it. With no
    /// value, prints the current default.
    GlobalProxy {
        /// Proxy URL, `direct`, or empty to clear. Omit to print the current value.
        proxy: Option<String>,
        /// Clear the global default (each server falls back to env, then direct).
        #[arg(long)]
        clear: bool,
    },
    /// Show a profile (token not shown).
    Show { name: Option<String> },
}

/// Trim a proxy value; an empty string becomes `None` (clear / inherit), mirroring
/// the GUI's `normalize_proxy` so both entry points behave identically.
fn normalize_proxy(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

pub async fn run(cmd: ServerCmd, config_flag: &Option<PathBuf>) -> Result<(), Error> {
    let path = super::config_path(config_flag);
    let mut cfg = Config::load(&path)?;
    match cmd {
        ServerCmd::List => {
            let out: Vec<_> = cfg
                .servers
                .iter()
                .map(|(name, p)| {
                    serde_json::json!({
                        "name": name,
                        "display_name": p.display(name),
                        "base_url": p.base_url,
                        "backend": p.backend,
                        "default": cfg.default_server.as_deref() == Some(name),
                        "enabled": p.enabled,
                        "poll_secs": p.poll_secs,
                        "proxy": p.proxy,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        ServerCmd::Add {
            name,
            url,
            backend,
            display_name,
            proxy,
            timeout,
            verify_ssl,
            poll_secs,
            disabled,
            default,
            force,
        } => {
            if !force && cfg.servers.contains_key(&name) {
                return Err(Error::Usage(format!(
                    "server '{name}' already exists; use --force to replace it"
                )));
            }
            let backend = match backend.as_str() {
                "openproject" => Backend::OpenProject,
                "github" => Backend::Github,
                other => {
                    return Err(Error::Usage(format!(
                        "unknown backend '{other}'; expected openproject|github"
                    )))
                }
            };
            let replaced = cfg
                .servers
                .insert(
                    name.clone(),
                    ServerProfile {
                        display_name,
                        backend,
                        base_url: url,
                        timeout,
                        verify_ssl,
                        proxy,
                        enabled: !disabled,
                        poll_secs,
                        timelog_start: None,
                        status_colors: Default::default(),
                        status_filters: Vec::new(),
                        display_fields: Vec::new(),
                        open_content_in: None,
                    },
                )
                .is_some();
            if default || cfg.default_server.is_none() {
                cfg.default_server = Some(name.clone());
            }
            cfg.save(&path)?;
            if replaced {
                println!("replaced server '{name}'");
            } else {
                println!("added server '{name}'");
            }
        }
        ServerCmd::Remove { name } => {
            if cfg.servers.remove(&name).is_none() {
                return Err(Error::Usage(format!("unknown server '{name}'")));
            }
            if cfg.default_server.as_deref() == Some(name.as_str()) {
                cfg.default_server = cfg.servers.keys().next().cloned();
            }
            cfg.save(&path)?;
            Secrets::resolve().delete(&name)?;
            println!("removed server '{name}'");
        }
        ServerCmd::Rename { old, new } => {
            if old == new {
                return Ok(());
            }
            let Some(profile) = cfg.servers.remove(&old) else {
                return Err(Error::Usage(format!("unknown server '{old}'")));
            };
            if cfg.servers.contains_key(&new) {
                // Put it back so the config is unchanged on error.
                cfg.servers.insert(old, profile);
                return Err(Error::Usage(format!("server '{new}' already exists")));
            }
            cfg.servers.insert(new.clone(), profile);
            if cfg.default_server.as_deref() == Some(old.as_str()) {
                cfg.default_server = Some(new.clone());
            }
            cfg.save(&path)?;
            // Move the stored token from the old key to the new one.
            let secrets = Secrets::resolve();
            if let Some(token) = secrets.get(&old)? {
                secrets.set(&new, &token)?;
                secrets.delete(&old)?;
            }
            println!("renamed server '{old}' to '{new}'");
        }
        ServerCmd::SetDefault { name } => {
            if !cfg.servers.contains_key(&name) {
                return Err(Error::Usage(format!("unknown server '{name}'")));
            }
            cfg.default_server = Some(name.clone());
            cfg.save(&path)?;
            println!("default server is now '{name}'");
        }
        ServerCmd::StatusColor {
            server,
            status,
            color,
            clear,
        } => {
            let name = cfg.resolve_server_name(server.as_deref())?;
            let profile = cfg.servers.get_mut(&name).expect("resolved server exists");
            match color {
                Some(token) if !clear => {
                    let parsed = StatusColor::from_token(&token).ok_or_else(|| {
                        Error::Usage(format!(
                            "unknown color '{token}'; expected danger|warn|success|progress|dimmed"
                        ))
                    })?;
                    profile.status_colors.insert(status.clone(), parsed);
                    cfg.save(&path)?;
                    println!("status '{status}' on '{name}' -> {token}");
                }
                _ => {
                    if profile.status_colors.remove(&status).is_some() {
                        cfg.save(&path)?;
                        println!("cleared color for status '{status}' on '{name}'");
                    } else {
                        println!("no color set for status '{status}' on '{name}'");
                    }
                }
            }
        }
        ServerCmd::Proxy {
            server,
            proxy,
            clear,
        } => {
            let name = cfg.resolve_server_name(server.as_deref())?;
            if proxy.is_none() && !clear {
                let current = cfg.servers[&name].proxy.as_deref().unwrap_or("");
                println!("{current}");
            } else {
                let value = if clear { None } else { normalize_proxy(proxy) };
                let profile = cfg.servers.get_mut(&name).expect("resolved server exists");
                profile.proxy = value.clone();
                cfg.save(&path)?;
                match value {
                    Some(v) => println!("proxy for '{name}' -> {v}"),
                    None => println!("cleared proxy override for '{name}'"),
                }
            }
        }
        ServerCmd::GlobalProxy { proxy, clear } => {
            if proxy.is_none() && !clear {
                println!("{}", cfg.proxy.as_deref().unwrap_or(""));
            } else {
                let value = if clear { None } else { normalize_proxy(proxy) };
                cfg.proxy = value.clone();
                cfg.save(&path)?;
                match value {
                    Some(v) => println!("global proxy -> {v}"),
                    None => println!("cleared global proxy"),
                }
            }
        }
        ServerCmd::Show { name } => {
            let name = cfg.resolve_server_name(name.as_deref())?;
            let p = &cfg.servers[&name];
            println!("{}", serde_json::to_string_pretty(p).unwrap());
        }
    }
    Ok(())
}
