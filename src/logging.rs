use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::fs::{File, OpenOptions, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const DEFAULT_LOG_DIR: &str = "/tmp/prime-agent";
const DEFAULT_LOG_FILE: &str = "prime-agent.log";

#[derive(Clone)]
pub struct Logger {
    verbose: bool,
    file: Arc<Mutex<File>>,
    log_path: PathBuf,
}

impl Logger {
    pub fn new(verbose: bool) -> Result<Self> {
        let log_dir = Path::new(DEFAULT_LOG_DIR);
        create_dir_all(log_dir).context("failed to create log directory")?;
        let log_path = log_dir.join(DEFAULT_LOG_FILE);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .context("failed to open log file")?;
        Ok(Self {
            verbose,
            file: Arc::new(Mutex::new(file)),
            log_path,
        })
    }

    #[must_use]
    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    /// Log a lifecycle step header in green and append to file.
    pub fn log_step(&self, message: &str) {
        println!("{}", message.green());
        self.write_line(message);
    }

    /// Log a substep message in dark gray when verbose and append to file.
    pub fn log_substep(&self, message: &str) {
        if self.verbose {
            println!("{}", message.bright_black());
        }
        self.write_line(message);
    }

    /// Log command output lines to file and optionally to console.
    pub fn log_output(&self, line: &str) {
        if self.verbose {
            println!("{}", line.bright_black());
        }
        self.write_line(line);
    }

    /// Log errors to stderr and append to file.
    pub fn log_error(&self, message: &str) {
        eprintln!("{message}");
        self.write_line(message);
    }

    /// Log error details with verbose output enforced.
    pub fn log_error_verbose(&self, message: &str, details: &[String]) {
        eprintln!("{message}");
        self.write_line(message);
        for line in details {
            eprintln!("{line}");
            self.write_line(line);
        }
    }

    fn write_line(&self, line: &str) {
        if let Ok(mut file) = self.file.lock() {
            let _ = writeln!(file, "{line}");
        }
    }
}
