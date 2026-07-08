pub mod auth;
pub mod import;
pub mod server;

use std::path::PathBuf;

use taskstream_core::config::default_config_path;

/// Resolve the effective config path from the global `--config` flag.
pub fn config_path(flag: &Option<PathBuf>) -> PathBuf {
    flag.clone().unwrap_or_else(default_config_path)
}
