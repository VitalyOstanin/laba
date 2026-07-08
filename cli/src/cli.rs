use clap::{Parser, Subcommand};

const AFTER_LONG_HELP: &str = "\
EXAMPLES:
  taskstream server add primary --url https://host/openproject
  taskstream auth login --server primary
  taskstream --server primary auth status
";

#[derive(Debug, Parser)]
#[command(name = "taskstream", version, after_long_help = AFTER_LONG_HELP)]
pub struct Cli {
    /// Server profile to use (overrides OPENPROJECT_SERVER and the default).
    #[arg(long, global = true, env = "OPENPROJECT_SERVER")]
    pub server: Option<String>,
    /// Override the token for this invocation.
    #[arg(long, global = true, env = "OPENPROJECT_TOKEN")]
    pub token: Option<String>,
    /// Override the proxy for this invocation (`none` disables it).
    #[arg(long, global = true, env = "OPENPROJECT_PROXY")]
    pub proxy: Option<String>,
    /// Path to config.json (defaults to the XDG config location).
    #[arg(long, global = true)]
    pub config: Option<std::path::PathBuf>,
    /// Max retries for idempotent requests.
    #[arg(long, global = true, env = "OPENPROJECT_RETRIES", default_value_t = 3)]
    pub retries: u32,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage server profiles.
    #[command(subcommand)]
    Server(crate::commands::server::ServerCmd),
    /// Authentication.
    #[command(subcommand)]
    Auth(crate::commands::auth::AuthCmd),
}
