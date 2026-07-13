use std::io::Read;
use std::path::PathBuf;

use clap::Subcommand;
use laba_core::auth::login_and_store;
use laba_core::client::Client;
use laba_core::config::Config;
use laba_core::error::Error;
use laba_core::secrets::Secrets;

#[derive(Debug, Subcommand)]
pub enum AuthCmd {
    /// Store a token for a server. Read the token from stdin (--with-token) or
    /// from the global --token flag. There is no interactive prompt: piping the
    /// token via stdin keeps it out of the process list and shell history.
    ///
    /// Login is online: the token is validated against `users/me` and the
    /// resolved account is used to reject a duplicate (another profile with the
    /// same base URL authenticated as the same user). Pass --force to add anyway.
    Login {
        /// Read the token from stdin.
        #[arg(long)]
        with_token: bool,
        /// Add even if another profile is the same user on the same base URL.
        #[arg(long)]
        force: bool,
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

/// Read the login token from `--token` or, with `--with-token`, from stdin.
/// There is no interactive prompt, so one of the two must be supplied.
fn read_login_token(token_flag: Option<&str>, with_token: bool) -> Result<String, Error> {
    if let Some(t) = token_flag {
        Ok(t.to_owned())
    } else if with_token {
        let mut s = String::new();
        std::io::stdin()
            .read_to_string(&mut s)
            .map_err(|e| Error::Io(e.to_string()))?;
        Ok(s.trim().to_owned())
    } else {
        Err(Error::Usage(
            "provide the token via stdin (--with-token) or --token".into(),
        ))
    }
}

/// Read the token (stdin/flag) and delegate to the shared core login, which
/// validates against `users/me`, rejects a duplicate account, and stores it.
async fn login(
    cfg: &Config,
    secrets: &Secrets,
    name: &str,
    token_flag: Option<&str>,
    with_token: bool,
    force: bool,
) -> Result<(), Error> {
    let token = read_login_token(token_flag, with_token)?;
    login_and_store(cfg, secrets, name, &token, force).await?;
    eprintln!("token stored for '{name}'");
    Ok(())
}

pub async fn run(
    cmd: AuthCmd,
    config_flag: &Option<PathBuf>,
    server_flag: Option<&str>,
    token_flag: Option<&str>,
) -> Result<(), Error> {
    let path = super::config_path(config_flag);
    let cfg = Config::load(&path)?;
    let secrets = Secrets::resolve();

    match cmd {
        AuthCmd::Login { with_token, force } => {
            let name = active_server(&cfg, server_flag)?;
            login(&cfg, &secrets, &name, token_flag, with_token, force).await
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
