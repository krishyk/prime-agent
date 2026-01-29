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
    pub workdir: PathBuf,
}

pub struct NextAction<'a> {
    pub step: &'a PlanStep,
    pub lifecycle: u8,
}

pub fn next_action<'a>(
    steps: &'a StepsFile,
    state: &StateFile,
    lifecycle_override: Option<u8>,
) -> Result<Option<NextAction<'a>>> {
    if let Some(lifecycle) = lifecycle_override {
        let (current_state, _, _) = lifecycle_mapping(lifecycle)?;
        let step = steps
            .steps
            .iter()
            .find(|step| state.state_for(&step.id) == current_state);
        return Ok(step.map(|step| NextAction { step, lifecycle }));
    }

    for step in &steps.steps {
        match state.state_for(&step.id) {
            StepState::Planned => return Ok(Some(NextAction { step, lifecycle: 1 })),
            StepState::Implemented => return Ok(Some(NextAction { step, lifecycle: 2 })),
            StepState::ImplementedChecked => return Ok(Some(NextAction { step, lifecycle: 3 })),
            StepState::ImplementedTested => return Ok(Some(NextAction { step, lifecycle: 4 })),
            StepState::ImplementedFinalized => return Ok(Some(NextAction { step, lifecycle: 5 })),
            StepState::LifecycleError(lifecycle_stage) => {
                if !(1..=5).contains(&lifecycle_stage) {
                    return Err(anyhow!("invalid lifecycle error stage: {lifecycle_stage}"));
                }
                return Ok(Some(NextAction {
                    step,
                    lifecycle: lifecycle_stage,
                }));
            }
            StepState::ImplementedCommitted => {}
        }
    }

    Ok(None)
}

/// Execute a single lifecycle step and update state.
///
/// # Errors
///
/// Returns an error if the agent action or gating commands fail.
pub fn run_lifecycle(
    config: &Config,
    plan: &Plan,
    state: &mut StateFile,
    options: &RunOptions,
    logger: &Logger,
    step: &PlanStep,
    lifecycle: u8,
) -> Result<bool> {
    let (current_state, next_state, action_label) = lifecycle_mapping(lifecycle)?;
    let step_state = state.state_for(&step.id);
    if step_state != current_state && step_state != StepState::LifecycleError(lifecycle) {
        return Err(anyhow!(
            "step {} in state '{}' cannot run lifecycle {}",
            step.id,
            step_state.label(),
            lifecycle
        ));
    }

    if plan.steps.iter().all(|plan_step| plan_step.id != step.id) {
        return Err(anyhow!(
            "step {} not found in plan, steps.json may be out of sync",
            step.id
        ));
    }
    logger.log_step(&format!(
        "Lifecycle {}: {} (step {}: {})",
        lifecycle, action_label, step.number, step.text
    ));

    logger.log_substep(&format!("State file: {}", options.state_path.display()));
    logger.log_substep(&format!("Workdir: {}", options.workdir.display()));
    let model = config.model_for(lifecycle);
    logger.log_substep(&format!("Using model: {model}"));
    logger.log_substep(&format!("Log file: {}", logger.log_path().display()));

    let diff_path = if lifecycle >= 2 {
        Some(
            write_git_diff(&options.workdir, logger)
                .with_context(|| "failed to capture git diff")?,
        )
    } else {
        None
    };

    let execution_result = if lifecycle == 5 {
        run_gates(config, options, logger)
            .and_then(|()| run_git_commit(step, options, logger, lifecycle))
            .with_context(|| format!("lifecycle {lifecycle}: git commit failed"))
    } else {
        let action = ActionContext {
            step,
            model: &model,
            action: action_label,
            lifecycle,
            diff_path: diff_path.as_deref(),
        };
        run_cli_action(config, options, logger, &action)
            .and_then(|()| run_gates(config, options, logger))
            .with_context(|| format!("lifecycle {lifecycle}: agent action failed"))
    }
    .with_context(|| {
        format!(
            "step {} (id {}) action {}",
            step.number, step.id, action_label
        )
    });

    if let Err(err) = execution_result {
        let details = vec![
            format!("Lifecycle: {}", lifecycle),
            format!("Action: {}", action_label),
            format!("Step ID: {}", step.id),
            format!("Step number: {}", step.number),
            format!("Step text: {}", step.text),
            format!("Workdir: {}", options.workdir.display()),
            format!("State file: {}", options.state_path.display()),
        ];
        state.set_state(&step.id, StepState::lifecycle_error(lifecycle));
        if let Err(save_err) = state.save(&options.state_path) {
            logger.log_error(&format!("Failed to save error state: {save_err}"));
        }
        logger.log_error_verbose(&format!("Lifecycle {lifecycle} failed: {err}"), &details);
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

struct ActionContext<'a> {
    step: &'a PlanStep,
    model: &'a str,
    action: &'static str,
    lifecycle: u8,
    diff_path: Option<&'a Path>,
}

fn run_cli_action(
    config: &Config,
    options: &RunOptions,
    logger: &Logger,
    action: &ActionContext<'_>,
) -> Result<()> {
    let (programs, args) = build_tool_command(config, options, action);
    run_command_with_fallback(
        &programs,
        &args,
        Some(&options.workdir),
        logger,
        &format!("agent action ({})", action.action),
    )
    .with_context(|| {
        let lifecycle = action.lifecycle;
        format!("failed to run agent tool for lifecycle {lifecycle}")
    })
}

fn build_tool_command(
    config: &Config,
    options: &RunOptions,
    action: &ActionContext<'_>,
) -> (Vec<String>, Vec<String>) {
    let tool_type = config.tool_type.unwrap_or(ToolType::Cursor);
    let programs = config.resolve_programs();
    let prompt = build_prompt(
        action.step,
        action.action,
        options,
        action.diff_path,
        action.lifecycle,
    );
    let mut args = config.cli_args.clone();

    match tool_type {
        ToolType::Cursor => {
            args.push("--print".to_string());
            args.push("--output-format".to_string());
            args.push("text".to_string());
            args.push("--model".to_string());
            args.push(action.model.to_string());
            args.push("--workspace".to_string());
            args.push(options.workdir.display().to_string());
            args.push("--force".to_string());
            args.push(prompt);
        }
        ToolType::Opencode => {
            args.push("run".to_string());
            args.push("--model".to_string());
            args.push(action.model.to_string());
            if let Some(diff) = action.diff_path {
                args.push("--file".to_string());
                args.push(diff.display().to_string());
            }
            args.push("--file".to_string());
            args.push(options.plan_path.display().to_string());
            args.push(prompt);
        }
    }

    (programs, args)
}

fn build_prompt(
    step: &PlanStep,
    action: &str,
    options: &RunOptions,
    diff_path: Option<&Path>,
    lifecycle: u8,
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
        lifecycle = lifecycle,
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
        run_command_with_fallback(
            std::slice::from_ref(&gate.command),
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

fn run_command_with_fallback(
    programs: &[String],
    args: &[String],
    workdir: Option<&Path>,
    logger: &Logger,
    label: &str,
) -> Result<()> {
    let mut last_error = None;
    let path_env = std::env::var("PATH").unwrap_or_else(|_| "<missing>".to_string());
    let workdir_display =
        workdir.map_or_else(|| "<none>".to_string(), |path| path.display().to_string());
    for program in programs {
        match run_command_once(program, args, workdir, logger, label) {
            Ok(()) => return Ok(()),
            Err(CommandError::NotFound(not_found)) => {
                logger.log_error_verbose(
                    "Command not found",
                    &[
                        format!("Program: {not_found}"),
                        format!("Label: {label}"),
                        format!("Workdir: {workdir_display}"),
                        format!("PATH: {path_env}"),
                    ],
                );
                last_error = Some(CommandError::NotFound(not_found));
            }
            Err(CommandError::Other(err)) => {
                return Err(err).with_context(|| format!("command execution failed: {label}"));
            }
        }
    }

    match last_error {
        Some(CommandError::NotFound(_)) => Err(anyhow!(
            "none of the candidate commands were found on PATH: {}",
            programs.join(", ")
        ))
        .with_context(|| format!("command candidates: {}", programs.join(", "))),
        Some(CommandError::Other(err)) => Err(err),
        None => Err(anyhow!("no command candidates provided")),
    }
}

enum CommandError {
    NotFound(String),
    Other(anyhow::Error),
}

fn run_command_once(
    program: &str,
    args: &[String],
    workdir: Option<&Path>,
    logger: &Logger,
    label: &str,
) -> Result<(), CommandError> {
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
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(CommandError::NotFound(program.to_string()));
        }
        Err(err) => {
            return Err(CommandError::Other(
                anyhow!(err).context(format!("failed to run command: {program}")),
            ));
        }
    };

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner())
            .tick_strings(&["|", "/", "-", "\\"]),
    );
    spinner.set_message(format!("Running {label}"));
    spinner.enable_steady_tick(Duration::from_millis(120));

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| CommandError::Other(anyhow!("missing stdout")))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| CommandError::Other(anyhow!("missing stderr")))?;
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

    let status = child
        .wait()
        .map_err(|err| CommandError::Other(anyhow!(err).context("failed waiting for command")))?;
    let _ = stdout_handle.join();
    let _ = stderr_handle.join();
    spinner.finish_and_clear();

    if !status.success() {
        let stderr_tail = format!("Exit status: {status}");
        let command_line = format!("{program} {}", args.join(" "));
        let detail = vec![
            format!("Program: {program}"),
            format!("Label: {label}"),
            format!("Args: {}", args.join(" ")),
            format!("Command: {command_line}"),
            stderr_tail,
        ];
        logger.log_error_verbose("Command failed", &detail);
        return Err(CommandError::Other(anyhow!(
            "command failed ({label}): {status}"
        )));
    }

    Ok(())
}

fn run_git_commit(
    step: &PlanStep,
    options: &RunOptions,
    logger: &Logger,
    lifecycle: u8,
) -> Result<()> {
    let message = format!(
        "stage implemented-finalized: step {} - {}",
        step.number, step.text
    );
    run_command_with_fallback(
        &["git".to_string()],
        &["add".to_string(), ".".to_string()],
        Some(&options.workdir),
        logger,
        "git add",
    )?;
    run_command_with_fallback(
        &["git".to_string()],
        &["commit".to_string(), "-m".to_string(), message],
        Some(&options.workdir),
        logger,
        &format!("git commit (lifecycle {lifecycle})"),
    )
}
