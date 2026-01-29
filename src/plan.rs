use anyhow::{anyhow, Context, Result};
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::state::{StateFile, StepState};

/// Parsed plan with ordered steps.
#[derive(Debug, Clone)]
pub struct Plan {
    pub steps: Vec<PlanStep>,
}

/// A single plan step parsed from Markdown.
#[derive(Debug, Clone)]
pub struct PlanStep {
    pub id: String,
    pub number: usize,
    pub text: String,
}

impl Plan {
    /// Load and parse a Markdown plan file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or no steps are found.
    pub fn load(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read plan file: {}", path.display()))?;
        let step_re = Regex::new(r"^\s*(\d+)\.\s+(.+?)\s*$")
            .context("failed to compile plan step regex")?;
        let mut steps = Vec::new();
        let mut seen_numbers = HashSet::new();

        for line in contents.lines() {
            if let Some(captures) = step_re.captures(line) {
                let number: usize = captures[1].parse().context("failed to parse step number")?;
                let text = captures[2].trim().to_string();
                if !seen_numbers.insert(number) {
                    return Err(anyhow!("duplicate plan step number: {number}"));
                }
                steps.push(PlanStep {
                    id: number.to_string(),
                    number,
                    text,
                });
            }
        }

        if steps.is_empty() {
            return Err(anyhow!("no steps found in plan file"));
        }

        Ok(Self { steps })
    }

    /// Return the next step matching the desired state.
    #[must_use]
    pub fn next_step_with_state<'a>(
        &'a self,
        state: &StateFile,
        desired: StepState,
    ) -> Option<&'a PlanStep> {
        self.steps
            .iter()
            .find(|step| state.state_for(&step.id) == desired)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parses_numbered_steps() {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        writeln!(file, "1. First step\n2. Second step").expect("write plan");
        let plan = Plan::load(file.path()).expect("load plan");
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].text, "First step");
        assert_eq!(plan.steps[1].id, "2");
    }
}
