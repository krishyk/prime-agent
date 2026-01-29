#![deny(warnings)]
#![warn(clippy::all, clippy::pedantic)]

mod config;
mod lifecycle;
mod logging;
mod plan;
mod state;
mod steps;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use crate::config::Config;
use crate::lifecycle::{RunOptions, run_lifecycle};
use crate::logging::Logger;
use crate::plan::Plan;
use crate::state::StateFile;
use crate::steps::StepsFile;

#[derive(Parser, Debug)]
#[command(
    name = "prime-agent",
    version,
    about = "Agent runner for lifecycle steps"
)]
struct Cli {
    /// Path to the Markdown plan file
    plan_path: PathBuf,
    /// Path to the JSON config file
    #[arg(long)]
    config: Option<PathBuf>,
    /// Lifecycle step to execute (1-5)
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=5))]
    lifecycle: u8,
    /// Enable verbose substep output
    #[arg(short, long)]
    verbose: bool,
    /// Path to the state JSON file
    #[arg(long)]
    state: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load_optional(cli.config.as_deref())?;
    let plan = Plan::load(&cli.plan_path)?;
    let steps = StepsFile::load_or_sync(&cli.plan_path, &plan)?;
    let state_path = cli
        .state
        .clone()
        .unwrap_or_else(|| PathBuf::from("state.json"));
    let mut state = StateFile::load(&state_path)?;
    let logger = Logger::new(cli.verbose)?;
    let workdir = cli
        .plan_path
        .parent()
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    let options = RunOptions {
        plan_path: cli.plan_path,
        state_path: state_path.clone(),
        lifecycle: cli.lifecycle,
        verbose: cli.verbose,
        workdir,
    };

    loop {
        let changed = run_lifecycle(&config, &steps, &plan, &mut state, &options, &logger)?;
        if changed {
            state.save(&state_path)?;
        } else {
            break;
        }
    }
    Ok(())
}
