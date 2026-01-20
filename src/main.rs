// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! CK - Intelligent Git Commit Assistant
//!
//! A production-grade CLI tool for creating high-quality Git commits.

use ck::cli::{run, Cli};
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn main() {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Set up logging
    setup_logging(cli.debug);

    // Run the CLI
    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

/// Set up logging/tracing.
fn setup_logging(debug: bool) {
    let filter = if debug {
        EnvFilter::try_new("ck=debug,warn").unwrap_or_else(|_| EnvFilter::new("warn"))
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    if debug {
        tracing::debug!("Debug logging enabled");
    }
}
