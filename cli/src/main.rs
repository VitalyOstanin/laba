use std::process::ExitCode;

use clap::Parser;
use taskstream_core::Error;

mod cli;
mod commands;

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
    match cli.command {
        cli::Command::Server(cmd) => commands::server::run(cmd, &cli.config).await,
        cli::Command::Auth(cmd) => {
            commands::auth::run(
                cmd,
                &cli.config,
                cli.server.as_deref(),
                cli.token.as_deref(),
            )
            .await
        }
    }
}
