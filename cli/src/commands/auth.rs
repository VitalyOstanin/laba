use std::io::Read;
use std::path::PathBuf;

use clap::Subcommand;
use taskstream_core::client::Client;
use taskstream_core::config::{Backend, Config};
use taskstream_core::error::Error;
use taskstream_core::secrets::Secrets;

#[derive(Debug, Subcommand)]
pub enum AuthCmd {
    /// Store a token for a server. Read the token from stdin (--with-token) or
    /// from the global --token flag. There is no interactive prompt: piping the
    /// token via stdin keeps it out of the process list and shell history.
    Login {
        /// Read the token from stdin.
        #[arg(long)]
        with_token: bool,
    },
    /// Show authentication status (optionally offline).
    Status {
        #[arg(long)]
        offline: bool,
    },
    /// Print the stored token.
    Token,
    /// Remove the stored token for the active server.
    Logout,
}

fn active_server(cfg: &Config, flag: Option<&str>) -> Result<String, Error> {
    cfg.resolve_server_name(flag)
}

pub async fn run(
    cmd: AuthCmd,
    config_flag: &Option<PathBuf>,
    server_flag: Option<&str>,
    token_flag: Option<&str>,
) -> Result<(), Error> {
    let path = super::config_path(config_flag);
    let cfg = Config::load(&path)?;
    let secrets = Secrets::new(Secrets::default_fallback_path());

    match cmd {
        AuthCmd::Login { with_token } => {
            let name = active_server(&cfg, server_flag)?;
            if cfg.servers[&name].backend == Backend::Github {
                return Err(Error::Usage(
                    "the github backend authenticates via gh; run 'gh auth login' instead".into(),
                ));
            }
            let token = if let Some(t) = token_flag {
                t.to_owned()
            } else if with_token {
                let mut s = String::new();
                std::io::stdin()
                    .read_to_string(&mut s)
                    .map_err(|e| Error::Io(e.to_string()))?;
                s.trim().to_owned()
            } else {
                return Err(Error::Usage(
                    "provide the token via stdin (--with-token) or --token".into(),
                ));
            };
            if token.is_empty() {
                return Err(Error::Usage("empty token".into()));
            }
            secrets.set(&name, &token)?;
            eprintln!("token stored for '{name}'");
            Ok(())
        }
        AuthCmd::Token => {
            let name = active_server(&cfg, server_flag)?;
            let tok = secrets
                .get(&name)?
                .ok_or_else(|| Error::Auth(format!("no token for '{name}'")))?;
            println!("{tok}");
            Ok(())
        }
        AuthCmd::Logout => {
            let name = active_server(&cfg, server_flag)?;
            secrets.delete(&name)?;
            eprintln!("logged out of '{name}'");
            Ok(())
        }
        AuthCmd::Status { offline } => {
            let name = active_server(&cfg, server_flag)?;
            let profile = &cfg.servers[&name];
            let token = token_flag
                .map(str::to_owned)
                .or(secrets.get(&name)?)
                .ok_or_else(|| Error::Auth(format!("no token for '{name}'")))?;
            if offline {
                println!(
                    "{}",
                    serde_json::json!({"server": name, "base_url": profile.base_url, "hasToken": true})
                );
                return Ok(());
            }
            let client = Client::new(&name, profile, token, None)?;
            let me = client.get_json_retrying("users/me", 3).await?;
            println!(
                "{}",
                serde_json::json!({"server": name, "loggedIn": true, "userId": me.get("id")})
            );
            Ok(())
        }
    }
}
