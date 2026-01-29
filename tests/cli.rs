use assert_cmd::cargo::cargo_bin_cmd;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tempfile::TempDir;

#[derive(Debug, Deserialize)]
struct StateFile {
    steps: HashMap<String, String>,
}

#[test]
fn runs_lifecycle_and_updates_state() {
    let temp = TempDir::new().expect("temp dir");
    let plan_path = temp.path().join("plan.md");
    let config_path = temp.path().join("config.json");
    let state_path = temp.path().join("state.json");
    let cli_path = temp.path().join("fake-cli.sh");

    fs::write(&plan_path, "1. First step\n2. Second step\n").expect("write plan");
    write_script(&cli_path, "#!/bin/sh\necho \"fake cli\" >&2\nexit 0\n");
    fs::write(
        &config_path,
        format!(
            r#"{{
            "cli-program": "unused",
            "tool-type": "cursor",
            "tool-paths": {{
                "cursor": "{}"
            }},
            "gates": [
                {{ "name": "noop", "command": "true", "args": [] }}
            ]
        }}"#,
            cli_path.display()
        ),
    )
    .expect("write config");

    let mut cmd = cargo_bin_cmd!("prime-agent");
    cmd.current_dir(temp.path())
        .arg(&plan_path)
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(&state_path)
        .arg("--lifecycle")
        .arg("1");
    cmd.assert().success();

    let state_contents = fs::read_to_string(&state_path).expect("read state");
    let parsed: StateFile = serde_json::from_str(&state_contents).expect("parse state");
    assert_eq!(
        parsed.steps.get("1").map(String::as_str),
        Some("implemented")
    );
}

#[test]
fn records_error_state_on_failure() {
    let temp = TempDir::new().expect("temp dir");
    let plan_path = temp.path().join("plan.md");
    let config_path = temp.path().join("config.json");
    let state_path = temp.path().join("state.json");
    let cli_path = temp.path().join("fail-cli.sh");

    fs::write(&plan_path, "1. First step\n").expect("write plan");
    fs::write(
        &state_path,
        r#"{
            "steps": {
                "1": "implemented"
            }
        }"#,
    )
    .expect("write state");
    write_script(&cli_path, "#!/bin/sh\necho \"boom\"\nexit 2\n");
    fs::write(
        &config_path,
        format!(
            r#"{{
            "cli-program": "unused",
            "tool-type": "opencode",
            "tool-paths": {{
                "opencode": "{}"
            }},
            "gates": [
                {{ "name": "noop", "command": "true", "args": [] }}
            ]
        }}"#,
            cli_path.display()
        ),
    )
    .expect("write config");

    let mut cmd = cargo_bin_cmd!("prime-agent");
    cmd.current_dir(temp.path())
        .arg(&plan_path)
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(&state_path)
        .arg("--lifecycle")
        .arg("2");
    cmd.assert().failure();

    let state_contents = fs::read_to_string(&state_path).expect("read state");
    let parsed: StateFile = serde_json::from_str(&state_contents).expect("parse state");
    assert_eq!(
        parsed.steps.get("1").map(String::as_str),
        Some("lifecycle-error-2")
    );
}

fn write_script(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write script");
    let mut perms = fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("set permissions");
}
