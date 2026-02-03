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

1. Create a `.stack-sync.toml` (or `stack-sync.toml`) config file in your project directory:

```toml
host = "https://portainer.example.com"
endpoint_id = 2  # optional, default 2

[stacks.my-stack]
compose_file = "compose.yaml"
env_file = ".env"  # optional

[stacks.other-stack]
compose_file = "other/compose.yaml"
env_file = "other/.env"
endpoint_id = 3  # optional per-stack override
```

| Field                     | Description                                       | Required |
| ------------------------- | ------------------------------------------------- | -------- |
| `host`                    | Portainer instance URL                            | Yes      |
| `endpoint_id`             | Default Portainer environment/endpoint ID         | No       |
| `stacks.<name>`           | Stack definition — the key is the stack name      | Yes      |
| `compose_file`            | Path to the local Docker Compose file             | Yes      |
| `env_file`                | Path to the local `.env` file for stack variables | No       |
| `endpoint_id` (per-stack) | Override the top-level endpoint ID                | No       |

2. Deploy the stack:

```bash
stack-sync sync
```

This creates or updates all stacks defined in the config. To target specific stacks:

```bash
stack-sync sync my-stack
stack-sync sync my-stack other-stack
```

## Commands

### sync

Create or update stacks in Portainer using the local compose files and env vars.

```bash
stack-sync sync                            # sync all stacks
stack-sync sync my-stack                   # sync one stack
stack-sync sync my-stack other-stack       # sync specific stacks
stack-sync sync my-stack --dry-run         # preview changes
stack-sync sync -C /path/to/config.toml    # use a different config file
```

The config path defaults to the current directory, where it will automatically look for `.stack-sync.toml` first, then `stack-sync.toml`. File paths in the config (`compose_file`, `env_file`) are resolved relative to the config file's directory, not the working directory.

### view

Show the current state of stacks in Portainer.

```bash
stack-sync view                            # view all stacks
stack-sync view my-stack                   # view one stack
stack-sync view -C /path/to/config.toml    # use a different config file
```

### pull

Download a stack's compose file and environment variables from Portainer to local files. This command does not require a config file.

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
