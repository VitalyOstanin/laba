use clap::Subcommand;
use laba_core::config::BackendKind;
use laba_core::error::Error;
use laba_core::resources::notification;

use crate::cli::Globals;

#[derive(Debug, Subcommand)]
pub enum NotificationCmd {
    /// List notifications, newest first.
    List {
        #[arg(long, default_value_t = 1)]
        offset: i64,
        #[arg(long)]
        limit: Option<i64>,
    },
    /// Mark a notification as read.
    Read { id: i64 },
    /// Mark a notification as unread.
    Unread { id: i64 },
}

pub async fn run(cmd: NotificationCmd, g: &Globals) -> Result<(), Error> {
    let (_name, profile) = super::load_profile(g)?;
    if profile.backend == BackendKind::Github {
        return match cmd {
            NotificationCmd::List { .. } => {
                super::ensure_gh_ready(&profile)?;
                let items = laba_core::backend::list_notifications(&profile, None).await?;
                let v = serde_json::to_value(&items)
                    .map_err(|e| Error::Internal(format!("serialize notifications: {e}")))?;
                crate::output::emit(&v, g.human);
                Ok(())
            }
            _ => super::require_openproject(&profile, "'notification read/unread'"),
        };
    }
    let (_name, client) = super::build_client(g)?;
    let raw = g.raw;
    let out = match cmd {
        NotificationCmd::List { offset, limit } => {
            notification::list(&client, offset, limit, raw).await?
        }
        NotificationCmd::Read { id } => notification::read(&client, id).await?,
        NotificationCmd::Unread { id } => notification::unread(&client, id).await?,
    };
    crate::output::emit(&out, g.human);
    Ok(())
}
