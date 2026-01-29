use anyhow::{Context, Result, anyhow};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::config::{Config, GateCommand, ToolType};
use crate::logging::Logger;
use crate::plan::{Plan, PlanStep};
use crate::state::{StateFile, StepState};
use crate::steps::StepsFile;

/// Runtime options for a lifecycle execution.
pub struct RunOptions {
    pub plan_path: PathBuf,
    pub state_path: PathBuf,
    pub lifecycle: u8,
    pub workdir: PathBuf,
}

/// Execute a single lifecycle step and update state.
///
/// # Errors
///
/// Returns an error if the agent action or gating commands fail.
pub fn run_lifecycle(
    config: &Config,
    steps: &StepsFile,
    plan: &Plan,
    state: &mut StateFile,
    options: &RunOptions,
    logger: &Logger,
) -> Result<bool> {
    let (current_state, next_state, action_label) = lifecycle_mapping(options.lifecycle)?;
    let Some(step) = steps
        .steps
        .iter()
        .find(|step| state.state_for(&step.id) == current_state)
    else {
        logger.log_step(&format!(
            "No step found in state '{}' for lifecycle {}",
            current_state.label(),
            options.lifecycle
        ));
        return Ok(false);
    };

    if plan.steps.iter().all(|plan_step| plan_step.id != step.id) {
        return Err(anyhow!(
            "step {} not found in plan, steps.json may be out of sync",
            step.id
        ));
    }
    logger.log_step(&format!(
        "Lifecycle {}: {} (step {}: {})",
        options.lifecycle, action_label, step.number, step.text
    ));

    logger.log_substep(&format!("State file: {}", options.state_path.display()));
    logger.log_substep(&format!("Workdir: {}", options.workdir.display()));
    let model = config.model_for(options.lifecycle);
    logger.log_substep(&format!("Using model: {model}"));
    logger.log_substep(&format!("Log file: {}", logger.log_path().display()));

    let diff_path = if options.lifecycle >= 2 {
        Some(write_git_diff(&options.workdir, logger)?)
    } else {
        None
    };

    let execution_result = if options.lifecycle == 5 {
        run_gates(config, options, logger).and_then(|()| run_git_commit(step, options, logger))
    } else {
        run_cli_action(
            config,
            step,
            &model,
            action_label,
            options,
            diff_path.as_deref(),
            logger,
        )
        .and_then(|()| run_gates(config, options, logger))
    };

    if let Err(err) = execution_result {
        state.set_state(&step.id, StepState::lifecycle_error(options.lifecycle));
        if let Err(save_err) = state.save(&options.state_path) {
            logger.log_error(&format!("Failed to save error state: {save_err}"));
        }
        logger.log_error(&format!(
            "Lifecycle {lifecycle} failed: {err}",
            lifecycle = options.lifecycle
        ));
        return Err(err);
    }

    state.set_state(&step.id, next_state);

    if let Some(path) = diff_path {
        let _ = std::fs::remove_file(path);
    }

    Ok(true)
}

fn lifecycle_mapping(lifecycle: u8) -> Result<(StepState, StepState, &'static str)> {
    let mapping = match lifecycle {
        1 => (StepState::Planned, StepState::Implemented, "implement"),
        2 => (
            StepState::Implemented,
            StepState::ImplementedChecked,
            "validate",
        ),
        3 => (
            StepState::ImplementedChecked,
            StepState::ImplementedTested,
            "test",
        ),
        4 => (
            StepState::ImplementedTested,
            StepState::ImplementedFinalized,
            "finalize",
        ),
        5 => (
            StepState::ImplementedFinalized,
            StepState::ImplementedCommitted,
            "commit",
        ),
        _ => return Err(anyhow!("invalid lifecycle: {lifecycle}")),
    };
    Ok(mapping)
}

fn run_cli_action(
    config: &Config,
    step: &PlanStep,
    model: &str,
    action: &str,
    options: &RunOptions,
    diff_path: Option<&Path>,
    logger: &Logger,
) -> Result<()> {
    let (program, args) = build_tool_command(config, step, model, action, options, diff_path);
    run_command(
        &program,
        &args,
        Some(&options.workdir),
        logger,
        &format!("agent action ({action})"),
    )
}

fn build_tool_command(
    config: &Config,
    step: &PlanStep,
    model: &str,
    action: &str,
    options: &RunOptions,
    diff_path: Option<&Path>,
) -> (String, Vec<String>) {
    let tool_type = config.tool_type.unwrap_or(ToolType::Cursor);
    let program = config.resolve_program();
    let prompt = build_prompt(step, action, options, diff_path);
    let mut args = config.cli_args.clone();

    match tool_type {
        ToolType::Cursor => {
            args.push("-p".to_string());
            args.push(prompt);
            args.push("--mode".to_string());
            args.push("agent".to_string());
            args.push("--output-format".to_string());
            args.push("text".to_string());
            args.push("--model".to_string());
            args.push(model.to_string());
        }
        ToolType::Opencode => {
            args.push("run".to_string());
            args.push("--model".to_string());
            args.push(model.to_string());
            if let Some(diff) = diff_path {
                args.push("--file".to_string());
                args.push(diff.display().to_string());
            }
            args.push("--file".to_string());
            args.push(options.plan_path.display().to_string());
            args.push(prompt);
        }
    }

    (program, args)
}

fn build_prompt(
    step: &PlanStep,
    action: &str,
    options: &RunOptions,
    diff_path: Option<&Path>,
) -> String {
    let diff_info = diff_path.map_or_else(
        || "Diff file: none".to_string(),
        |path| format!("Diff file: {}", path.display()),
    );
    format!(
        "You are running non-interactively. Do not ask for confirmation. \
Action: {action}\n\
Lifecycle: {lifecycle}\n\
Plan file: {plan}\n\
Step ID: {step_id}\n\
Step text: {step_text}\n\
{diff_info}\n\
Execute the step, apply necessary changes, and exit.",
        action = action,
        lifecycle = options.lifecycle,
        plan = options.plan_path.display(),
        step_id = step.id,
        step_text = step.text
    )
}

fn run_gates(config: &Config, options: &RunOptions, logger: &Logger) -> Result<()> {
    logger.log_step("Gates: lint/build/test");
    let gates = if config.gates.is_empty() {
        default_gates()
    } else {
        config.gates.clone()
    };

    for gate in gates {
        let name = gate.name.unwrap_or_else(|| gate.command.clone());
        logger.log_substep(&format!("Running gate: {name}"));
        run_command(
            &gate.command,
            &gate.args,
            Some(&options.workdir),
            logger,
            &format!("gate: {name}"),
        )?;
    }

    Ok(())
}

fn default_gates() -> Vec<GateCommand> {
    vec![
        GateCommand {
            name: Some("fmt-check".to_string()),
            command: "cargo".to_string(),
            args: vec!["fmt".to_string(), "--".to_string(), "--check".to_string()],
        },
        GateCommand {
            name: Some("clippy".to_string()),
            command: "cargo".to_string(),
            args: vec![
                "clippy".to_string(),
                "--".to_string(),
                "-D".to_string(),
                "warnings".to_string(),
            ],
        },
        GateCommand {
            name: Some("build".to_string()),
            command: "cargo".to_string(),
            args: vec!["build".to_string()],
        },
        GateCommand {
            name: Some("test".to_string()),
            command: "cargo".to_string(),
            args: vec!["test".to_string()],
        },
    ]
}

fn write_git_diff(workdir: &Path, logger: &Logger) -> Result<PathBuf> {
    let diff_output = Command::new("git")
        .args(["diff"])
        .current_dir(workdir)
        .output()
        .context("failed to execute git diff")?;

    let diff_text = String::from_utf8_lossy(&diff_output.stdout).to_string();
    if diff_text.trim().is_empty() {
        logger.log_substep("git diff is empty");
    }

    let filename = format!(
        "prime-agent-diff-{}.patch",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let path = std::env::temp_dir().join(filename);
    let mut file = File::create(&path).context("failed to create diff temp file")?;
    file.write_all(diff_text.as_bytes())
        .context("failed to write diff temp file")?;
    Ok(path)
}

fn run_command(
    program: &str,
    args: &[String],
    workdir: Option<&Path>,
    logger: &Logger,
    label: &str,
) -> Result<()> {
    logger.log_substep(&format!(
        "Executing {}: {} {}",
        label,
        program,
        args.join(" ")
    ));

    let mut command = Command::new(program);
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(workdir) = workdir {
        command.current_dir(workdir);
    }
    let mut child = command
        .spawn()
        .with_context(|| format!("failed to run command: {program}"))?;

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner())
            .tick_strings(&["|", "/", "-", "\\"]),
    );
    spinner.set_message(format!("Running {label}"));
    spinner.enable_steady_tick(Duration::from_millis(120));

    let stdout = child.stdout.take().context("missing stdout")?;
    let stderr = child.stderr.take().context("missing stderr")?;
    let logger_stdout = logger.clone();
    let logger_stderr = logger.clone();

    let stdout_handle = std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            logger_stdout.log_output(&line);
        }
    });
    let stderr_handle = std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            logger_stderr.log_output(&line);
        }
    });

    let status = child.wait().context("failed waiting for command")?;
    let _ = stdout_handle.join();
    let _ = stderr_handle.join();
    spinner.finish_and_clear();

    if !status.success() {
        return Err(anyhow!("command failed ({label}): {status}"));
    }

    Ok(())
}

fn run_git_commit(step: &PlanStep, options: &RunOptions, logger: &Logger) -> Result<()> {
    let message = format!(
        "stage implemented-finalized: step {} - {}",
        step.number, step.text
    );
    run_command(
        "git",
        &["add".to_string(), ".".to_string()],
        Some(&options.workdir),
        logger,
        "git add",
    )?;
    run_command(
        "git",
        &["commit".to_string(), "-m".to_string(), message],
        Some(&options.workdir),
        logger,
        &format!("git commit (lifecycle {})", options.lifecycle),
    )
}
