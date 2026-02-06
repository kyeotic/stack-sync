# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.3.6] - 2026-02-05

### Added
- nix flake install instructions

### Changed
- `upgrade` handling when installed by nix

## [0.3.5] - 2026-02-05

### Added
- `--verbose`/`-V` flag for `sync`, `redeploy`, and `view` commands
- Detail lines (Host, Compose file, Env file, Endpoint ID, etc.) are now hidden by default and shown with `--verbose`

## [0.3.4] - 2026-02-05

### Changed
- All commands now use pretty-printed output

## [0.3.3] - 2026-02-03

### Added
- Homebrew tap release

## [0.3.2] - 2026-02-03

### Added
- `redeploy` command to force image re-pull and redeploy a stack

## [0.3.1] - 2026-02-03

### Changed
- `sync` change detection now ignores trialing newlines


## [0.3.0] - 2026-02-03

### Added
- `init` command to create config files for new projects
- `import` command to import existing stacks from Portainer

### Removed
- `pull` command (replaced by `import`)

## [0.2.1] - 2026-02-02
### Added
- Support for `.stack-sync.toml`

## [0.2.0] - 2026-02-02
### Changed
- Config file format, now supports multiple stacks
- `sync` and `view` updated to handle multiple stacks and