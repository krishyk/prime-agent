# Agent Guidance

## Build, Lint, Test Discipline

- Always run `cargo clippy --all-targets --all-features -- -D warnings`, fix any issues.
- Always run `cargo build`, fix any issues.
- Always run `cargo test`, fix any issues.

## Versioning

- Bump the patch version on every change and print it on every run of `prime-agent`.
- Do not write version files to target project directories at runtime.
