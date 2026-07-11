use std::io::Write;
use std::path::PathBuf;

use clap::Subcommand;
use laba_core::error::Error;
use laba_core::resources::attachment;

use crate::cli::Globals;

#[derive(Debug, Subcommand)]
pub enum AttachmentCmd {
    /// List the attachments of a work package.
    List {
        #[arg(long)]
        work_package: i64,
        #[arg(long, default_value_t = 1)]
        offset: i64,
        #[arg(long)]
        limit: Option<i64>,
    },
    /// Fetch a single attachment by id.
    Get { id: i64 },
    /// Download an attachment's content ('-o -' streams to stdout).
    Download {
        id: i64,
        #[arg(long, short = 'o')]
        output: String,
        #[arg(long)]
        max_bytes: Option<u64>,
    },
    /// Upload a file as an attachment on a work package.
    Upload {
        #[arg(long)]
        work_package: i64,
        file: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        content_type: Option<String>,
    },
    /// Delete an attachment by id.
    Delete { id: i64 },
}

pub async fn run(cmd: AttachmentCmd, g: &Globals) -> Result<(), Error> {
    let (_name, client) = super::build_client(g)?;
    let raw = g.raw;
    let out = match cmd {
        AttachmentCmd::List {
            work_package,
            offset,
            limit,
        } => attachment::list(&client, work_package, offset, limit, raw).await?,
        AttachmentCmd::Get { id } => attachment::get(&client, id, raw).await?,
        AttachmentCmd::Download {
            id,
            output,
            max_bytes,
        } => {
            if output == "-" {
                let stdout = std::io::stdout();
                let mut lock = stdout.lock();
                let info = client
                    .stream_download(&format!("attachments/{id}/content"), &mut lock)
                    .await?;
                match lock.flush() {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
                    Err(e) => return Err(Error::Io(format!("flush stdout: {e}"))),
                }
                eprintln!("streamed {} bytes", info.bytes);
                return Ok(());
            }
            attachment::download(&client, id, &PathBuf::from(output), max_bytes).await?
        }
        AttachmentCmd::Upload {
            work_package,
            file,
            name,
            description,
            content_type,
        } => {
            attachment::upload(
                &client,
                work_package,
                &file,
                name.as_deref(),
                description.as_deref(),
                content_type.as_deref(),
                raw,
            )
            .await?
        }
        AttachmentCmd::Delete { id } => attachment::delete(&client, id).await?,
    };
    crate::output::emit(&out, g.human);
    Ok(())
}
