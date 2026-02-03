# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.3.0] - 2026-02-03

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