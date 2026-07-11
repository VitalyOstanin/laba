use clap::builder::PossibleValuesParser;
use clap::Subcommand;
use laboro_core::error::Error;
use laboro_core::resources::relation::{self, RELATION_TYPES};

use crate::cli::Globals;

#[derive(Debug, Subcommand)]
pub enum RelationCmd {
    /// List the relations of a work package.
    List {
        #[arg(long)]
        work_package: i64,
        #[arg(long, value_parser = PossibleValuesParser::new(RELATION_TYPES))]
        type_: Option<String>,
        #[arg(long, default_value_t = 1)]
        offset: i64,
        #[arg(long)]
        limit: Option<i64>,
    },
    /// Fetch a single relation by id.
    Get { id: i64 },
    /// Create a relation from a work package to another.
    Create {
        #[arg(long)]
        work_package: i64,
        #[arg(long)]
        to: i64,
        #[arg(long = "type", value_parser = PossibleValuesParser::new(RELATION_TYPES))]
        type_: String,
        #[arg(long)]
        description: Option<String>,
    },
    /// Update a relation.
    Update {
        id: i64,
        #[arg(long = "type", value_parser = PossibleValuesParser::new(RELATION_TYPES))]
        type_: Option<String>,
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a relation by id.
    Delete { id: i64 },
}

pub async fn run(cmd: RelationCmd, g: &Globals) -> Result<(), Error> {
    let (_name, client) = super::build_client(g)?;
    let raw = g.raw;
    let out = match cmd {
        RelationCmd::List {
            work_package,
            type_,
            offset,
            limit,
        } => relation::list(&client, work_package, type_.as_deref(), offset, limit, raw).await?,
        RelationCmd::Get { id } => relation::get(&client, id, raw).await?,
        RelationCmd::Create {
            work_package,
            to,
            type_,
            description,
        } => {
            relation::create(
                &client,
                work_package,
                to,
                &type_,
                description.as_deref(),
                raw,
            )
            .await?
        }
        RelationCmd::Update {
            id,
            type_,
            description,
        } => relation::update(&client, id, type_.as_deref(), description.as_deref(), raw).await?,
        RelationCmd::Delete { id } => relation::delete(&client, id).await?,
    };
    crate::output::emit(&out, g.human);
    Ok(())
}
