use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Lifecycle state for a plan step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepState {
    Planned,
    Implemented,
    ImplementedChecked,
    ImplementedTested,
    ImplementedFinalized,
    ImplementedCommitted,
    LifecycleError(u8),
}

impl StepState {
    /// Human-readable label for logs.
    #[must_use]
    pub fn label(self) -> String {
        match self {
            StepState::Planned => "planned".to_string(),
            StepState::Implemented => "implemented".to_string(),
            StepState::ImplementedChecked => "implemented-checked".to_string(),
            StepState::ImplementedTested => "implemented-tested".to_string(),
            StepState::ImplementedFinalized => "implemented-finalized".to_string(),
            StepState::ImplementedCommitted => "implemented-committed".to_string(),
            StepState::LifecycleError(stage) => format!("lifecycle-error-{stage}"),
        }
    }

    #[must_use]
    pub fn lifecycle_error(stage: u8) -> Self {
        Self::LifecycleError(stage)
    }
}

impl Serialize for StepState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.label())
    }
}

impl<'de> Deserialize<'de> for StepState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "planned" => Ok(StepState::Planned),
            "implemented" => Ok(StepState::Implemented),
            "implemented-checked" => Ok(StepState::ImplementedChecked),
            "implemented-tested" => Ok(StepState::ImplementedTested),
            "implemented-finalized" => Ok(StepState::ImplementedFinalized),
            "implemented-committed" => Ok(StepState::ImplementedCommitted),
            _ => {
                if let Some(stage) = value.strip_prefix("lifecycle-error-") {
                    let parsed = stage
                        .parse::<u8>()
                        .map_err(|_| de::Error::custom("invalid lifecycle error stage"))?;
                    return Ok(StepState::LifecycleError(parsed));
                }
                Err(de::Error::custom("unknown step state"))
            }
        }
    }
}

/// State file that tracks each step's lifecycle.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StateFile {
    #[serde(default)]
    pub steps: HashMap<String, StepState>,
}

impl StateFile {
    /// Load state from JSON or return an empty state if missing.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read state file: {}", path.display()))?;
        let parsed: Self = serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse state JSON: {}", path.display()))?;
        Ok(parsed)
    }

    /// Persist the state to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create state dir: {}", parent.display()))?;
        }
        let contents = serde_json::to_string_pretty(self).context("failed to serialize state")?;
        fs::write(path, contents)
            .with_context(|| format!("failed to write state file: {}", path.display()))?;
        Ok(())
    }

    /// Get the state for a step, defaulting to planned.
    #[must_use]
    pub fn state_for(&self, step_id: &str) -> StepState {
        self.steps
            .get(step_id)
            .copied()
            .unwrap_or(StepState::Planned)
    }

    /// Update the state for a step.
    pub fn set_state(&mut self, step_id: &str, state: StepState) {
        self.steps.insert(step_id.to_string(), state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_loads_state() {
        let mut state = StateFile::default();
        state.set_state("1", StepState::Implemented);
        let file = tempfile::NamedTempFile::new().expect("temp file");
        state.save(file.path()).expect("save state");
        let loaded = StateFile::load(file.path()).expect("load state");
        assert_eq!(loaded.state_for("1"), StepState::Implemented);
        assert_eq!(loaded.state_for("2"), StepState::Planned);
    }

    #[test]
    fn serializes_error_state() {
        let state = StepState::LifecycleError(3);
        let json = serde_json::to_string(&state).expect("serialize");
        assert_eq!(json, "\"lifecycle-error-3\"");
        let parsed: StepState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, StepState::LifecycleError(3));
    }
}
