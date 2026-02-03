# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

stack-sync is a CLI tool for deploying and managing Portainer stacks from the terminal. It syncs local Docker Compose files and environment variables to Portainer, and can pull existing stacks down for local editing.

## Commands

```bash
just test          # Run all unit tests
just lint          # Clippy with warnings as errors
just fmt           # Format code
just fmt-check     # Check formatting
just check         # Quick syntax check
just build         # Optimized release build
just run [args]    # Run with arguments (e.g., just run sync)
```

## Architecture

Five modules with clear separation:

- **main.rs** — CLI parsing (clap derive), command dispatch, requires `PORTAINER_API_KEY` env var
- **config.rs** — TOML config loading; relative paths resolve against config file directory, not cwd
- **commands.rs** — Business logic for sync/view/pull; includes custom date algorithm (no chrono dependency) and ANSI color output
- **portainer.rs** — HTTP client for Portainer API; custom serde deserializer (`deserialize_null_as_default`) handles null API values; PascalCase API fields mapped to snake_case via serde rename
- **update.rs** — Self-update from GitHub releases; platform detection for macOS/Linux; uses self-replace for in-place binary swap

## Key Patterns

- `anyhow::Result<T>` with `.context()` for all error handling
- Unit tests in `#[cfg(test)]` blocks within each module
- Sync command checks for existing stack before deciding create vs update
- `--dry-run` flag on sync for preview without changes
- Rust edition 2024; release profile uses `strip = true` and `lto = true`
