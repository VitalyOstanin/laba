use std::io::Read;

use clap::Args;
use laboro_core::error::Error;
use laboro_core::resources::api;

use crate::cli::Globals;

#[derive(Debug, Args)]
pub struct ApiArgs {
    /// HTTP method (GET, POST, PATCH, DELETE, ...).
    pub method: String,
    /// API path, e.g. `work_packages/1`.
    pub path: String,
    /// String field `key=value` (repeatable).
    #[arg(short = 'f', long = "field")]
    pub field: Vec<String>,
    /// Typed field `key=value` (repeatable, parsed as JSON with string fallback).
    #[arg(short = 'F', long = "raw-field")]
    pub raw_field: Vec<String>,
    /// Read the request body from a file (or '-' for stdin).
    #[arg(long)]
    pub input: Option<String>,
}

/// Split a `key=value` field on the first `=`; error when no `=` is present.
fn parse_field(raw: &str) -> Result<(String, String), Error> {
    match raw.split_once('=') {
        Some((k, v)) => Ok((k.to_string(), v.to_string())),
        None => Err(Error::Usage(format!(
            "Invalid field '{raw}'; expected key=value."
        ))),
    }
}

pub async fn run(args: ApiArgs, g: &Globals) -> Result<(), Error> {
    let (_name, client) = super::build_client(g)?;

    let fields: Vec<(String, String)> = args
        .field
        .iter()
        .map(|f| parse_field(f))
        .collect::<Result<_, _>>()?;
    let raw_fields: Vec<(String, String)> = args
        .raw_field
        .iter()
        .map(|f| parse_field(f))
        .collect::<Result<_, _>>()?;

    let input = match args.input.as_deref() {
        None => None,
        Some("-") => {
            let mut s = String::new();
            std::io::stdin()
                .read_to_string(&mut s)
                .map_err(|e| Error::Io(e.to_string()))?;
            Some(s)
        }
        Some(path) => Some(
            std::fs::read_to_string(path).map_err(|e| Error::Io(format!("read {path}: {e}")))?,
        ),
    };

    let out = api::call(
        &client,
        &args.method,
        &args.path,
        &fields,
        &raw_fields,
        input.as_deref(),
    )
    .await?;
    crate::output::emit(&out, g.human);
    Ok(())
}
