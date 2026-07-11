use clap::Subcommand;
use serde_json::Value;
use taskstream_core::client::Client;
use taskstream_core::config::{Backend, ServerProfile};
use taskstream_core::error::Error;
use taskstream_core::resources::work_packages::{self, WpFields, WpListParams};

use crate::cli::Globals;

#[derive(Debug, Subcommand)]
pub enum WpCmd {
    /// List work packages by filters.
    List {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long = "type")]
        type_: Option<String>,
        #[arg(long)]
        assignee: Option<String>,
        #[arg(long)]
        subject: Option<String>,
        #[arg(long)]
        open: bool,
        #[arg(long)]
        include_past: bool,
        #[arg(long, default_value_t = 1)]
        offset: i64,
        #[arg(long)]
        limit: Option<i64>,
    },
    /// Fetch a single work package by id.
    Get { id: i64 },
    /// Full-text search across work packages.
    Search {
        text: String,
        #[arg(long, default_value_t = 1)]
        offset: i64,
        #[arg(long)]
        limit: Option<i64>,
    },
    /// Run a saved query by id.
    Query {
        id: i64,
        #[arg(long, default_value_t = 1)]
        offset: i64,
        #[arg(long)]
        limit: Option<i64>,
    },
    /// Create a work package.
    Create {
        #[arg(long)]
        project: String,
        #[arg(long = "type")]
        type_: String,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        assignee: Option<String>,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        start_date: Option<String>,
        #[arg(long)]
        due_date: Option<String>,
        #[arg(long)]
        done_ratio: Option<i64>,
    },
    /// Update a work package.
    Update {
        id: i64,
        #[arg(long)]
        subject: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long = "type")]
        type_: Option<String>,
        #[arg(long)]
        assignee: Option<String>,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        start_date: Option<String>,
        #[arg(long)]
        due_date: Option<String>,
        #[arg(long)]
        done_ratio: Option<i64>,
    },
    /// Delete a work package.
    Delete { id: i64 },
}

/// GitHub backend: only `list` is supported (my open issues and pull requests,
/// filters not applied yet); other subcommands need write/detail APIs the github
/// path does not provide.
async fn run_github(cmd: WpCmd, profile: &ServerProfile, g: &Globals) -> Result<(), Error> {
    match cmd {
        WpCmd::List { .. } => {
            super::ensure_gh_ready(profile)?;
            let tasks = taskstream_core::backend::list_tasks(profile, None).await?;
            crate::output::emit(&Value::Array(tasks), g.human);
            Ok(())
        }
        _ => super::require_openproject(profile, "this 'wp' subcommand"),
    }
}

/// Dispatch an OpenProject `wp` subcommand to its resource operation, returning
/// the JSON value to emit.
async fn run_openproject(cmd: WpCmd, client: &Client, raw: bool) -> Result<Value, Error> {
    let out = match cmd {
        WpCmd::List {
            project,
            status,
            type_,
            assignee,
            subject,
            open,
            include_past,
            offset,
            limit,
        } => {
            let params = WpListParams {
                project,
                status,
                type_,
                assignee,
                subject,
                open,
                include_past,
                offset,
                limit,
            };
            work_packages::list(client, params, raw).await?
        }
        WpCmd::Get { id } => work_packages::get(client, id, raw).await?,
        WpCmd::Search {
            text,
            offset,
            limit,
        } => work_packages::search(client, &text, offset, limit, raw).await?,
        WpCmd::Query { id, offset, limit } => {
            work_packages::query(client, id, offset, limit, raw).await?
        }
        WpCmd::Create {
            project,
            type_,
            subject,
            description,
            status,
            assignee,
            parent,
            start_date,
            due_date,
            done_ratio,
        } => {
            let fields = WpFields {
                subject: Some(subject),
                description,
                start_date,
                due_date,
                done_ratio,
                project: Some(project),
                type_: Some(type_),
                status,
                assignee,
                parent,
            };
            work_packages::create(client, fields, raw).await?
        }
        WpCmd::Update {
            id,
            subject,
            description,
            status,
            type_,
            assignee,
            parent,
            start_date,
            due_date,
            done_ratio,
        } => {
            let fields = WpFields {
                subject,
                description,
                start_date,
                due_date,
                done_ratio,
                project: None,
                type_,
                status,
                assignee,
                parent,
            };
            work_packages::update(client, id, fields, raw).await?
        }
        WpCmd::Delete { id } => work_packages::delete(client, id).await?,
    };
    Ok(out)
}

pub async fn run(cmd: WpCmd, g: &Globals) -> Result<(), Error> {
    let (_name, profile) = super::load_profile(g)?;
    if profile.backend == Backend::Github {
        return run_github(cmd, &profile, g).await;
    }
    let (_name, client) = super::build_client(g)?;
    let out = run_openproject(cmd, &client, g.raw).await?;
    crate::output::emit(&out, g.human);
    Ok(())
}
