use std::path::PathBuf;

use clap::Subcommand;
use serde_json::json;
use taskstream_core::cache::Cache;
use taskstream_core::config::Config;
use taskstream_core::error::Error;

#[derive(Debug, Subcommand)]
pub enum CacheCmd {
    /// Clear cached stable entities.
    Clear {
        /// Clear a specific server (defaults to the active one).
        #[arg(long)]
        server: Option<String>,
        /// Clear caches of all servers.
        #[arg(long)]
        all: bool,
    },
}

pub async fn run(
    cmd: CacheCmd,
    config_flag: &Option<PathBuf>,
    server_flag: Option<&str>,
    human: bool,
) -> Result<(), Error> {
    match cmd {
        CacheCmd::Clear { server, all } => {
            if all {
                Cache::clear_all()?;
                crate::output::emit(&json!({"cleared": "all"}), human);
            } else {
                let path = super::config_path(config_flag);
                let cfg = Config::load(&path)?;
                let name = cfg.resolve_server_name(server.as_deref().or(server_flag))?;
                Cache::clear_server(&name)?;
                crate::output::emit(&json!({"cleared": name}), human);
            }
        }
    }
    Ok(())
}
