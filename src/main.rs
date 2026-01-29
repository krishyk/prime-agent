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
use crate::lifecycle::{RunOptions, next_action, run_lifecycle};
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
    /// Lifecycle override to execute (1-5)
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=5))]
    lifecycle: Option<u8>,
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
    let current_dir = std::env::current_dir()?;
    let plan_path = if cli.plan_path.is_absolute() {
        cli.plan_path
    } else {
        current_dir.join(cli.plan_path)
    };
    let state_path = cli
        .state
        .clone()
        .unwrap_or_else(|| PathBuf::from(".prime-agent/state.json"));
    let mut state = StateFile::load(&state_path)?;
    let logger = Logger::new(cli.verbose)?;
    logger.log_step(&format!(
        "prime-agent version {}",
        env!("CARGO_PKG_VERSION")
    ));
    let workdir = plan_path
        .parent()
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    let options = RunOptions {
        plan_path,
        state_path: state_path.clone(),
        workdir,
    };

    loop {
        let plan = Plan::load(&options.plan_path)?;
        let (steps, synced) = StepsFile::load_or_sync(&options.plan_path, &plan)?;
        if synced {
            logger.log_substep("steps.json synced with plan.md");
        }
        let Some(next) = next_action(&steps, &state, cli.lifecycle)? else {
            break;
        };
        let changed = run_lifecycle(
            &config,
            &plan,
            &mut state,
            &options,
            &logger,
            next.step,
            next.lifecycle,
        )?;
        if changed {
            state.save(&state_path)?;
        } else {
            break;
        }
    }
    Ok(())
}
