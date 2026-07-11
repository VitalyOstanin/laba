use clap::Subcommand;
use laboro_core::error::Error;
use laboro_core::resources::time;

use crate::cli::Globals;

#[derive(Debug, Subcommand)]
pub enum TimeCmd {
    /// List time entries.
    List {
        #[arg(long)]
        user: Option<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        work_package: Option<i64>,
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        until: Option<String>,
        #[arg(long, default_value_t = 1)]
        offset: i64,
        #[arg(long)]
        limit: Option<i64>,
    },
    /// Fetch a single time entry by id.
    Get { id: i64 },
    /// Create a time entry against a work package.
    Create {
        #[arg(long)]
        work_package: i64,
        #[arg(long)]
        hours: Option<f64>,
        #[arg(long)]
        duration: Option<String>,
        #[arg(long)]
        spent_on: Option<String>,
        #[arg(long)]
        comment: Option<String>,
        #[arg(long)]
        activity: Option<String>,
    },
    /// Update an existing time entry.
    Update {
        id: i64,
        #[arg(long)]
        hours: Option<f64>,
        #[arg(long)]
        duration: Option<String>,
        #[arg(long)]
        spent_on: Option<String>,
        #[arg(long)]
        comment: Option<String>,
        #[arg(long)]
        activity: Option<String>,
    },
    /// Delete a time entry.
    Delete { id: i64 },
}

pub async fn run(cmd: TimeCmd, g: &Globals) -> Result<(), Error> {
    let (_name, client) = super::build_client(g)?;
    let raw = g.raw;
    let out = match cmd {
        TimeCmd::List {
            user,
            project,
            work_package,
            since,
            until,
            offset,
            limit,
        } => {
            time::list(
                &client,
                user.as_deref(),
                project.as_deref(),
                work_package,
                since.as_deref(),
                until.as_deref(),
                offset,
                limit,
                raw,
            )
            .await?
        }
        TimeCmd::Get { id } => time::get(&client, id, raw).await?,
        TimeCmd::Create {
            work_package,
            hours,
            duration,
            spent_on,
            comment,
            activity,
        } => {
            time::create(
                &client,
                work_package,
                hours,
                duration.as_deref(),
                spent_on.as_deref(),
                comment.as_deref(),
                activity.as_deref(),
                raw,
            )
            .await?
        }
        TimeCmd::Update {
            id,
            hours,
            duration,
            spent_on,
            comment,
            activity,
        } => {
            time::update(
                &client,
                id,
                hours,
                duration.as_deref(),
                spent_on.as_deref(),
                comment.as_deref(),
                activity.as_deref(),
                raw,
            )
            .await?
        }
        TimeCmd::Delete { id } => time::delete(&client, id).await?,
    };
    crate::output::emit(&out, g.human);
    Ok(())
}
