use clap::Subcommand;
use taskstream_core::error::Error;
use taskstream_core::resources::comment;

use crate::cli::Globals;

#[derive(Debug, Subcommand)]
pub enum CommentCmd {
    /// List the activities of a work package.
    List {
        #[arg(long)]
        work_package: i64,
        #[arg(long)]
        comments_only: bool,
        #[arg(long, default_value_t = 1)]
        offset: i64,
        #[arg(long)]
        limit: Option<i64>,
    },
    /// Fetch a single activity by id.
    Get { id: i64 },
    /// Create a comment on a work package.
    Create {
        #[arg(long)]
        work_package: i64,
        text: String,
    },
    /// Update an existing comment.
    Update { id: i64, text: String },
}

pub async fn run(cmd: CommentCmd, g: &Globals) -> Result<(), Error> {
    let (_name, client) = super::build_client(g)?;
    let raw = g.raw;
    let out = match cmd {
        CommentCmd::List {
            work_package,
            comments_only,
            offset,
            limit,
        } => comment::list(&client, work_package, comments_only, offset, limit, raw).await?,
        CommentCmd::Get { id } => comment::get(&client, id, raw).await?,
        CommentCmd::Create { work_package, text } => {
            comment::create(&client, work_package, &text, raw).await?
        }
        CommentCmd::Update { id, text } => comment::update(&client, id, &text, raw).await?,
    };
    crate::output::emit(&out, g.human);
    Ok(())
}
