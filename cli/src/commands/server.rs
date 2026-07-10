use std::path::PathBuf;

use clap::Subcommand;
use taskstream_core::config::{Config, ServerProfile, StatusColor};
use taskstream_core::error::Error;
use taskstream_core::secrets::Secrets;

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
    /// Show a profile (token not shown).
    Show { name: Option<String> },
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
            let replaced = cfg
                .servers
                .insert(
                    name.clone(),
                    ServerProfile {
                        display_name,
                        backend: Default::default(),
                        base_url: url,
                        timeout,
                        verify_ssl,
                        proxy,
                        enabled: !disabled,
                        poll_secs,
                        timelog_start: None,
                        status_colors: Default::default(),
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
            Secrets::new(Secrets::default_fallback_path()).delete(&name)?;
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
            let secrets = Secrets::new(Secrets::default_fallback_path());
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
            if clear || color.is_none() {
                if profile.status_colors.remove(&status).is_some() {
                    cfg.save(&path)?;
                    println!("cleared color for status '{status}' on '{name}'");
                } else {
                    println!("no color set for status '{status}' on '{name}'");
                }
            } else {
                let token = color.unwrap();
                let parsed = StatusColor::from_token(&token).ok_or_else(|| {
                    Error::Usage(format!(
                        "unknown color '{token}'; expected danger|warn|success|dimmed"
                    ))
                })?;
                profile.status_colors.insert(status.clone(), parsed);
                cfg.save(&path)?;
                println!("status '{status}' on '{name}' -> {token}");
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
