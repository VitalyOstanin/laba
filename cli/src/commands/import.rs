use std::path::Path;

use clap::Args;
use taskstream_core::config::{Config, ServerProfile};
use taskstream_core::error::Error;
use taskstream_core::secrets::Secrets;

#[derive(Debug, Args)]
pub struct ImportArgs {
    /// Name for the imported profile.
    #[arg(long, default_value = "imported")]
    pub name: String,
    /// Path to the taskstream-cli config.yaml (defaults to its XDG location).
    #[arg(long)]
    pub from: Option<std::path::PathBuf>,
}

pub async fn run(args: ImportArgs, config_path: &Path) -> Result<(), Error> {
    let src = args.from.unwrap_or_else(default_taskstream_cli_config);
    let text = std::fs::read_to_string(&src).map_err(|e| {
        Error::Usage(format!(
            "cannot read taskstream-cli config {}: {e}",
            src.display()
        ))
    })?;
    // taskstream-cli stores `url:` (and optionally an insecure `token:`) in YAML.
    // Parse the two keys line-wise to avoid a YAML dependency for a one-shot import.
    let url = scan_yaml_scalar(&text, "url")
        .ok_or_else(|| Error::Usage("no 'url' in taskstream-cli config".into()))?;
    let token = scan_yaml_scalar(&text, "token");

    let mut cfg = Config::load(config_path)?;
    cfg.servers.insert(
        args.name.clone(),
        ServerProfile {
            base_url: url,
            timeout: 30,
            verify_ssl: true,
            proxy: None,
        },
    );
    if cfg.default_server.is_none() {
        cfg.default_server = Some(args.name.clone());
    }
    cfg.save(config_path)?;

    if let Some(tok) = token {
        Secrets::new(Secrets::default_fallback_path()).set(&args.name, &tok)?;
        println!("imported server '{}' with token", args.name);
    } else {
        println!(
            "imported server '{}' (no token in source; run 'auth login')",
            args.name
        );
    }
    Ok(())
}

fn default_taskstream_cli_config() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| std::path::PathBuf::from(".config"));
    base.join("taskstream-cli").join("config.yaml")
}

/// Extract a top-level scalar `key: value` from simple YAML (quotes trimmed).
fn scan_yaml_scalar(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(key) {
            if let Some(val) = rest.trim_start().strip_prefix(':') {
                let v = val.trim().trim_matches(['"', '\'']).to_owned();
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_url_and_token() {
        let y = "url: \"https://h/openproject\"\ntimeout: 30\ntoken: abc\n";
        assert_eq!(
            scan_yaml_scalar(y, "url").as_deref(),
            Some("https://h/openproject")
        );
        assert_eq!(scan_yaml_scalar(y, "token").as_deref(), Some("abc"));
        assert_eq!(scan_yaml_scalar(y, "missing"), None);
    }
}
