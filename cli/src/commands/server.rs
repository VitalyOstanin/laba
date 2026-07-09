use std::path::PathBuf;

use clap::Subcommand;
use taskstream_core::config::{Config, ServerProfile};
use taskstream_core::error::Error;
use taskstream_core::secrets::Secrets;

#[derive(Debug, Subcommand)]
pub enum ServerCmd {
    /// List server profiles.
    List,
    /// Add a server profile.
    Add {
        name: String,
        #[arg(long)]
        url: String,
        #[arg(long)]
        proxy: Option<String>,
        #[arg(long, default_value_t = 30)]
        timeout: u64,
        #[arg(long, default_value_t = true)]
        verify_ssl: bool,
        /// Mark this profile as the default.
        #[arg(long)]
        default: bool,
        /// Replace an existing profile with the same name.
        #[arg(long)]
        force: bool,
    },
    /// Remove a server profile (and its stored token).
    Remove { name: String },
    /// Set the default server.
    SetDefault { name: String },
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
                        "base_url": p.base_url,
                        "default": cfg.default_server.as_deref() == Some(name),
                        "proxy": p.proxy,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        ServerCmd::Add {
            name,
            url,
            proxy,
            timeout,
            verify_ssl,
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
                        backend: Default::default(),
                        base_url: url,
                        timeout,
                        verify_ssl,
                        proxy,
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
        ServerCmd::SetDefault { name } => {
            if !cfg.servers.contains_key(&name) {
                return Err(Error::Usage(format!("unknown server '{name}'")));
            }
            cfg.default_server = Some(name.clone());
            cfg.save(&path)?;
            println!("default server is now '{name}'");
        }
        ServerCmd::Show { name } => {
            let name = cfg.resolve_server_name(name.as_deref())?;
            let p = &cfg.servers[&name];
            println!("{}", serde_json::to_string_pretty(p).unwrap());
        }
    }
    Ok(())
}
