use clap::{Parser, Subcommand};

const AFTER_LONG_HELP: &str = "\
EXAMPLES:
  laba server add primary --url https://host/openproject
  laba auth login --server primary
  laba --server primary auth status
";

#[derive(Debug, Parser)]
#[command(name = "laba", version, after_long_help = AFTER_LONG_HELP)]
pub struct Cli {
    #[command(flatten)]
    pub globals: Globals,

    #[command(subcommand)]
    pub command: Command,
}

/// Global flags shared by every subcommand.
#[derive(Debug, clap::Args)]
pub struct Globals {
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
    /// IANA timezone for date defaults and datetime display (e.g. Europe/Moscow;
    /// defaults to the machine-local zone). Matches the GUI's timezone setting.
    #[arg(long, global = true, env = "LABA_TZ")]
    pub tz: Option<String>,
    /// Increase logging: -v logs request method/URL/status/timing (debug), -vv
    /// also logs request/response bodies (trace). RUST_LOG overrides this.
    #[arg(short = 'v', long = "verbose", global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,
    /// Human-friendly output instead of JSON.
    #[arg(long, global = true)]
    pub human: bool,
    /// Raw API response without normalization.
    #[arg(long, global = true)]
    pub raw: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage server profiles.
    #[command(subcommand)]
    Server(crate::commands::server::ServerCmd),
    /// Authentication.
    #[command(subcommand)]
    Auth(crate::commands::auth::AuthCmd),
    /// Manage cached stable entities.
    #[command(subcommand)]
    Cache(crate::commands::cache::CacheCmd),
    /// Work packages.
    #[command(subcommand)]
    Wp(crate::commands::wp::WpCmd),
    /// Comments (work package activities).
    #[command(subcommand)]
    Comment(crate::commands::comment::CommentCmd),
    /// Attachments.
    #[command(subcommand)]
    Attachment(crate::commands::attachment::AttachmentCmd),
    /// Relations between work packages.
    #[command(subcommand)]
    Relation(crate::commands::relation::RelationCmd),
    /// Time entries.
    #[command(subcommand)]
    Time(crate::commands::time::TimeCmd),
    /// Notifications.
    #[command(subcommand)]
    Notification(crate::commands::notification::NotificationCmd),
    /// Raw API passthrough.
    Api(crate::commands::api::ApiArgs),
}
