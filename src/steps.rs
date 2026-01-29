use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::plan::{Plan, PlanStep};

const STEPS_FILE_NAME: &str = "steps.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct StepsFile {
    pub steps: Vec<PlanStep>,
}

impl StepsFile {
    pub fn load_or_sync(plan_path: &Path, plan: &Plan) -> Result<(Self, bool)> {
        let steps_path = Self::path_for(plan_path);
        if steps_path.exists() {
            let contents = fs::read_to_string(&steps_path)
                .with_context(|| format!("failed to read steps file: {}", steps_path.display()))?;
            let parsed: Self = serde_json::from_str(&contents)
                .with_context(|| format!("failed to parse steps file: {}", steps_path.display()))?;
            if parsed.steps == plan.steps {
                return Ok((parsed, false));
            }
        }

        let steps = Self {
            steps: plan.steps.clone(),
        };
        steps.save(&steps_path)?;
        Ok((steps, true))
    }

    #[must_use]
    pub fn path_for(plan_path: &Path) -> PathBuf {
        plan_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(STEPS_FILE_NAME)
    }

    fn save(&self, path: &Path) -> Result<()> {
        let contents =
            serde_json::to_string_pretty(self).context("failed to serialize steps.json")?;
        fs::write(path, contents)
            .with_context(|| format!("failed to write steps file: {}", path.display()))?;
        Ok(())
    }
}
