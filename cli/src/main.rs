use std::process::ExitCode;

use clap::Parser;
use laba_core::Error;

mod cli;
mod commands;
mod output;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("laba: {e}");
            ExitCode::from(e.exit_code())
        }
    }
}

/// Install the logger. `RUST_LOG` wins when set; otherwise `-v`/`-vv` raise the
/// level (warn by default, debug at `-v`, trace at `-vv`). Records go to stderr
/// so stdout stays clean for JSON output.
fn init_logging(verbose: u8) {
    let mut builder = env_logger::Builder::from_env(env_logger::Env::default());
    if std::env::var_os("RUST_LOG").is_none() {
        let level = match verbose {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        };
        builder.filter_level(level);
    }
    builder.init();
}

/// Remove in-flight download temp files if the process is interrupted, then
/// exit. The cleanup and exit run on a dedicated thread (not in async-signal
/// context), so the filesystem/lock work in `cleanup_temp_downloads` is safe.
/// Best-effort: a failure to install the handler leaves the prior (no-cleanup)
/// behavior.
///
/// On Unix the process exits with the conventional `128 + signum` code so
/// callers can tell interruptions apart: SIGINT -> 130, SIGTERM -> 143,
/// SIGHUP -> 129.
#[cfg(unix)]
fn install_signal_cleanup() {
    use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
    use signal_hook::iterator::Signals;

    let mut signals = match Signals::new([SIGINT, SIGTERM, SIGHUP]) {
        Ok(s) => s,
        Err(_) => return,
    };
    std::thread::spawn(move || {
        if let Some(signum) = signals.forever().next() {
            laba_core::client::cleanup_temp_downloads();
            std::process::exit(128 + signum);
        }
    });
}

/// Windows has no SIGTERM/SIGHUP; `ctrlc` maps Ctrl-C / Ctrl-Break to one
/// handler. Exit 130 (the SIGINT-equivalent code) after cleanup.
#[cfg(not(unix))]
fn install_signal_cleanup() {
    let _ = ctrlc::set_handler(|| {
        laba_core::client::cleanup_temp_downloads();
        std::process::exit(130);
    });
}

async fn run() -> Result<(), Error> {
    let cli = cli::Cli::parse();
    let g = &cli.globals;
    init_logging(g.verbose);
    install_signal_cleanup();
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
