use std::process::ExitCode;

use clap::Parser;
use taskstream_core::Error;

mod cli;
mod commands;
mod output;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("taskstream: {e}");
            ExitCode::from(e.exit_code())
        }
    }
}

async fn run() -> Result<(), Error> {
    let cli = cli::Cli::parse();
    let g = &cli.globals;
    match cli.command {
        cli::Command::Server(cmd) => commands::server::run(cmd, &g.config).await,
        cli::Command::Auth(cmd) => {
            commands::auth::run(cmd, &g.config, g.server.as_deref(), g.token.as_deref()).await
        }
        cli::Command::Cache(cmd) => {
            commands::cache::run(cmd, &g.config, g.server.as_deref(), g.human).await
        }
        cli::Command::Wp(cmd) => commands::wp::run(cmd, g).await,
        cli::Command::Comment(cmd) => commands::comment::run(cmd, g).await,
        cli::Command::Attachment(cmd) => commands::attachment::run(cmd, g).await,
        cli::Command::Relation(cmd) => commands::relation::run(cmd, g).await,
        cli::Command::Time(cmd) => commands::time::run(cmd, g).await,
        cli::Command::Notification(cmd) => commands::notification::run(cmd, g).await,
        cli::Command::Api(args) => commands::api::run(args, g).await,
    }
}
