use std::io::Read;
use std::path::PathBuf;

use clap::Subcommand;
use laba_core::client::Client;
use laba_core::config::{Backend, Config, ServerProfile};
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

/// Stable account identity from a `users/me` payload: the login if present,
/// otherwise the numeric id rendered as a string.
fn identity(me: &serde_json::Value) -> Option<String> {
    me.get("login")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .or_else(|| me.get("id").map(|v| v.to_string()))
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

/// The name of an existing server on the same base URL authenticated as the same
/// account `id`, if any. Profiles whose identity cannot be confirmed (missing or
/// expired token, unreachable host) cannot prove a duplicate and are skipped.
async fn duplicate_server(
    cfg: &Config,
    secrets: &Secrets,
    name: &str,
    profile: &ServerProfile,
    id: &str,
) -> Result<Option<String>, Error> {
    for (other, op) in &cfg.servers {
        if other == name || op.base_url != profile.base_url {
            continue;
        }
        let Some(other_token) = secrets.get(other)? else {
            continue;
        };
        let oc = Client::new(other, op, other_token, None)?;
        if let Ok(other_me) = oc.get_json_retrying("users/me", 1).await {
            if identity(&other_me).as_deref() == Some(id) {
                return Ok(Some(other.clone()));
            }
        }
    }
    Ok(None)
}

/// Validate a token against `users/me` and store it, rejecting a duplicate
/// (another profile with the same base URL and account) unless `force`.
async fn login(
    cfg: &Config,
    secrets: &Secrets,
    name: &str,
    token_flag: Option<&str>,
    with_token: bool,
    force: bool,
) -> Result<(), Error> {
    let profile = &cfg.servers[name];
    if profile.backend == Backend::Github {
        return Err(Error::Usage(
            "the github backend authenticates via gh; run 'gh auth login' instead".into(),
        ));
    }
    let token = read_login_token(token_flag, with_token)?;
    if token.is_empty() {
        return Err(Error::Usage("empty token".into()));
    }
    // Validate the token and read the account identity, needed to detect a
    // duplicate (same base URL + same user).
    let client = Client::new(name, profile, token.clone(), None)?;
    let me = client.get_json_retrying("users/me", 3).await?;
    if !force {
        if let Some(id) = identity(&me) {
            if let Some(other) = duplicate_server(cfg, secrets, name, profile, &id).await? {
                return Err(Error::Usage(format!(
                    "user '{id}' at {} is already authenticated as server '{other}'; use --force to add anyway",
                    profile.base_url
                )));
            }
        }
    }
    secrets.set(name, &token)?;
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

#[cfg(test)]
mod tests {
    use super::identity;
    use serde_json::json;

    #[test]
    fn identity_prefers_login_then_falls_back_to_id() {
        assert_eq!(
            identity(&json!({"login": "alice", "id": 7})).as_deref(),
            Some("alice"),
        );
        assert_eq!(identity(&json!({"id": 7})).as_deref(), Some("7"),);
        assert_eq!(identity(&json!({})), None);
    }
}
