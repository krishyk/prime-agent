use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Runner configuration loaded from JSON.
#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(rename = "cli-program")]
    pub cli_program: String,
    #[serde(rename = "tool-type")]
    pub tool_type: Option<ToolType>,
    #[serde(rename = "tool-paths", default)]
    pub tool_paths: HashMap<ToolType, String>,
    #[serde(rename = "cli-args", default)]
    pub cli_args: Vec<String>,
    #[serde(default)]
    pub lifecycles: HashMap<String, LifecycleConfig>,
    #[serde(default)]
    pub gates: Vec<GateCommand>,
}

/// Configuration for a specific lifecycle.
#[derive(Debug, Deserialize, Default)]
pub struct LifecycleConfig {
    pub model: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ToolType {
    Cursor,
    Opencode,
}

/// Command definition for a gating step.
#[derive(Debug, Deserialize, Clone)]
pub struct GateCommand {
    pub name: Option<String>,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

impl Config {
    /// Load configuration from a JSON file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed as JSON.
    pub fn load(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let parsed: Self = serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse config JSON: {}", path.display()))?;
        Ok(parsed)
    }

    /// Load configuration from a JSON file or return defaults.
    ///
    /// # Errors
    ///
    /// Returns an error if the file is provided but cannot be read or parsed.
    pub fn load_optional(path: Option<&Path>) -> Result<Self> {
        match path {
            Some(path) => Self::load(path),
            None => Ok(Self::default()),
        }
    }

    /// Resolve the model name for a lifecycle with defaults.
    pub fn model_for(&self, lifecycle: u8) -> String {
        let key = lifecycle.to_string();
        let from_config = self
            .lifecycles
            .get(&key)
            .and_then(|config| config.model.clone());
        if let Some(model) = from_config {
            return model;
        }
        match lifecycle {
            2 | 4 | 5 => "opus 4.5 max mode".to_string(),
            3 => "codex 5.2 max mode".to_string(),
            _ => "gpt codex 5.2 max mode".to_string(),
        }
    }

    /// Resolve the CLI program path based on tool type and overrides.
    #[must_use]
    pub fn resolve_program(&self) -> String {
        if let Some(tool_type) = self.tool_type
            && let Some(path) = self.tool_paths.get(&tool_type)
        {
            return path.clone();
        }
        self.cli_program.clone()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cli_program: "cursor-agent".to_string(),
            tool_type: Some(ToolType::Cursor),
            tool_paths: HashMap::new(),
            cli_args: Vec::new(),
            lifecycles: HashMap::new(),
            gates: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_for_uses_override() {
        let json = r#"{
            "cli-program": "cursor",
            "lifecycles": { "1": { "model": "custom-model" } }
        }"#;
        let config: Config = serde_json::from_str(json).expect("valid config");
        assert_eq!(config.model_for(1), "custom-model");
        assert_eq!(config.model_for(2), "opus 4.5 max mode");
    }

    #[test]
    fn resolves_tool_type_program() {
        let json = r#"{
            "cli-program": "default-cli",
            "tool-type": "cursor",
            "tool-paths": {
                "cursor": "/tmp/cursor-cli",
                "opencode": "/tmp/opencode"
            }
        }"#;
        let config: Config = serde_json::from_str(json).expect("valid config");
        assert_eq!(config.resolve_program(), "/tmp/cursor-cli");
    }

    #[test]
    fn default_config_uses_cursor_agent() {
        let config = Config::default();
        assert_eq!(config.resolve_program(), "cursor-agent");
    }
}
