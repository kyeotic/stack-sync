# stack-sync

Deploy and manage [Portainer](https://www.portainer.io/) stacks from the command line. Sync local Docker Compose files and environment variables to Portainer, or pull existing stacks down for local editing.

## Prerequisites

- A Portainer instance with API access enabled
- A `PORTAINER_API_KEY` (create one in Portainer under **User Settings > Access Tokens**)

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/kyeotic/stack-sync/main/install.sh | bash
```

Or download a binary from the [releases page](https://github.com/kyeotic/stack-sync/releases).

## Authentication

stack-sync reads the `PORTAINER_API_KEY` environment variable for all API requests. Set it in your shell:

```bash
export PORTAINER_API_KEY=your-api-key-here
```

The key is sent as an `X-API-KEY` header. The Portainer endpoint ID is resolved automatically.

## Quick Start

1. Create a `stack-sync.toml` config file in your project directory:

```toml
name = "my-stack"
compose_file = "compose.yaml"
env_file = ".env"
host = "https://portainer.example.com"
```

| Field          | Description                                       |
| -------------- | ------------------------------------------------- |
| `name`         | Stack name in Portainer                           |
| `compose_file` | Path to the local Docker Compose file             |
| `env_file`     | Path to the local `.env` file for stack variables |
| `host`         | Portainer instance URL                            |

2. Deploy the stack:

```bash
stack-sync sync
```

This creates the stack if it doesn't exist, or updates it if it does.

## Commands

### sync

Create or update a stack in Portainer using the local compose file and env vars.

```bash
stack-sync sync [config-path]
```

The config path defaults to `stack-sync.toml` in the current directory. File paths in the config (`compose_file`, `env_file`) are resolved relative to the config file's directory, not the working directory.

### view

Show the current state of the stack in Portainer.

```bash
stack-sync view [config-path]
```

### pull

Download a stack's compose file and environment variables from Portainer to local files. This command does not require a `stack-sync.toml` config file.

```bash
stack-sync pull \
  --host https://portainer.example.com \
  --stack my-stack \
  --file compose.yaml \
  --env .env
```

| Argument  | Description                                |
| --------- | ------------------------------------------ |
| `--host`  | Portainer hostname                         |
| `--stack` | Name of the stack in Portainer             |
| `--file`  | Path to write the compose file to          |
| `--env`   | Path to write the environment variables to |

### upgrade

Check for the latest release and update the binary in-place.

```bash
stack-sync upgrade
```

## Env File Format

The `.env` file uses standard `KEY=value` format:

```
DATABASE_URL=postgres://localhost:5432/mydb
API_KEY=sk-abc123
DEBUG=true
```

Lines starting with `#` and blank lines are ignored.
