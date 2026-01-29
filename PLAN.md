## Project setup

- Initialize a new Rust workspace in `/home/prime/personal/prime-agent` with a single CLI crate and strict linting (`#![deny(warnings)]`, `clippy::all`, `clippy::pedantic`), and add dependencies for CLI parsing, JSON/Markdown parsing, logging, and test utilities.
- Create core modules: `src/main.rs`, `src/config.rs`, `src/plan.rs`, `src/state.rs`, `src/lifecycle.rs`, and `src/logging.rs`.

## Data formats and parsing

- Define the JSON config shape in `src/config.rs` matching `{"cli-program":"...","lifecycles":{"1":{"model":"..."}}}` with defaults for models per lifecycle when not specified.
- Define a Markdown plan format in `src/plan.rs` (e.g., headings with numbered steps and `state:` markers) and implement parsing to extract ordered steps and their states.
- Implement state persistence in `/home/prime/personal/prime-agent/state.json` via `src/state.rs`, mapping plan step IDs to states (`planned`, `implemented`, `implemented-checked`, `implemented-tested`, `implemented-finalized`).

## Lifecycle execution

- Implement lifecycle selection and step progression in `src/lifecycle.rs`:
  - Step 1: take next `planned` step, run Cursor CLI with chosen model, mark `implemented`.
  - Step 2: take next `implemented` step, validate git diff, fix if needed, mark `implemented-checked`.
  - Step 3: take next `implemented-checked` step, add unit + integration tests (Cloudflare local for web, process spawn for CLI/TUI), mark `implemented-tested`.
  - Step 4: take next `implemented-tested` step, validate tests cover feature, mark `implemented-finalized`.
  - Step 5: take next `implemented-finalized` step, `git add .` and `git commit`, mark `implemented-committed`.
- Add a gating pipeline before completing any lifecycle step: run lint/style, build, and tests, and block progression on failures (never delete tests unless functionality removed).
- On errors, immediately surface failures and set step state to `lifecycle-error-<lifecycle>` in `state.json` so the lifecycle can be retried.

## CLI and logging

- Implement CLI in `src/main.rs` with a positional plan path argument (`prime-agent <path/to/plan.md>`) plus flags: `--config <path>`, `--lifecycle <1|2|3|4>`, `--verbose`, and optional `--state <path>`.
- Implement colorized logging in `src/logging.rs`: lifecycle step headers in green, substep output in dark gray, and only show substep output when `-v` is set.
- Stream full command output to `/tmp/prime-agent/prime-agent.log` and show a throbber while commands are running.

## Tests

- Add unit tests for config parsing, plan parsing, and state transitions.
- Add integration tests for CLI behavior (mocking git and Cursor CLI interactions) and for lifecycle gating behavior.

## Validation

- Run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo build`, and `cargo test` to ensure lint-free and passing tests.
