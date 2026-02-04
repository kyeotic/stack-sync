# stack-sync

Deploy and manage [Portainer](https://www.portainer.io/) stacks from the command line. Sync local Docker Compose files and environment variables to Portainer, or import existing stacks for local editing.

## Prerequisites

- A Portainer instance with API access enabled
- A `PORTAINER_API_KEY` (create one in Portainer under **User Settings > Access Tokens**)

## Installation

### With Homebrew (tap)

```
brew install kyeotic/homebrew-stack-sync/stack-sync
```

### One-line Shell

```bash
curl -fsSL https://raw.githubusercontent.com/kyeotic/stack-sync/main/install.sh | bash
```

Or download a binary from the [releases page](https://github.com/kyeotic/stack-sync/releases).

## Quick Start

1. Create a `.stack-sync.toml` config file in your project directory:

```toml
host = "https://portainer.example.com"
endpoint_id = 2  # optional, default 2
portainer_api_key = "Your_Key"

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

## Splitting Configuration

Since creating a config file in your repo with secrets like the `portainer_api_key` is a bad practice, and since your also likely to share the `host` with several projects, stack-sync supports config inheritance.

These fields: `host`, `portainer_api_key` and `endpoint_id`, can be provided by a `.stack-sync.toml` in any parent directory up to and including your `$HOME` directory. This allows config to be stored outside the working directory.

Alternatively you can provide a `PORTAINER_API_KEY` as an ENV VAR (e.g. sourced from a .env file by [dir-env](https://direnv.net/)) and not put it any config files.

The order of precedence is

- `$PORTAINER_API_KEY` ENV VAR
- current config (or config provided by `--config`)
- The next parent directory

Configs can also be merged: if the parent directory config contains an `endpoint_id` and the `$HOME` directory config contains a `host` they will form a complete configuration,

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

### init

Initialize config files for a new project. Creates a parent config with credentials and a local config with an example stack.

```bash
stack-sync init \
  --portainer-api-key ptr_xxx \
  --host https://portainer.example.com \
  --endpoint-id 2 \
  --parent-dir ~
```

| Argument              | Description                                  | Required |
| --------------------- | -------------------------------------------- | -------- |
| `--portainer-api-key` | Portainer API key                            | Yes      |
| `--host`              | Portainer hostname                           | Yes      |
| `--endpoint-id`       | Default endpoint ID (defaults to 2)          | No       |
| `--parent-dir`        | Directory for credentials config (default ~) | No       |
| `--force`             | Overwrite existing files                     | No       |

This creates two files:
- `{parent-dir}/.stack-sync.toml` — credentials (api key, host, endpoint)
- `./.stack-sync.toml` — local config with example stack commented out

### import

Import an existing stack from Portainer into your local config. Downloads the compose file and env vars, and adds the stack to your config.

```bash
stack-sync import my-stack                    # import a stack
stack-sync import my-stack --force            # overwrite existing files
stack-sync import my-stack -C /path/to/dir    # use a different config directory
```

| Argument  | Description                      | Required |
| --------- | -------------------------------- | -------- |
| `<stack>` | Name of the stack in Portainer   | Yes      |
| `-C`      | Path to config file or directory | No       |
| `--force` | Overwrite existing files         | No       |

Creates `{stack}.compose.yaml` and `{stack}.env` files, and adds a `[stacks.{stack}]` entry to the local config.

### redeploy

Force a stack to re-pull images and redeploy. Useful after pushing new images to a local registry. This command uses the current configuration from Portainer (not local files) — it triggers a re-pull without changing the stack's compose file or environment variables.

```bash
stack-sync redeploy my-stack                   # redeploy a stack
stack-sync redeploy my-stack --dry-run         # preview what would happen
stack-sync redeploy my-stack -C /path/to/dir   # use a different config directory
```

| Argument    | Description                      | Required |
| ----------- | -------------------------------- | -------- |
| `<stack>`   | Name of the stack to redeploy    | Yes      |
| `-C`        | Path to config file or directory | No       |
| `--dry-run` | Preview without making changes   | No       |

The stack must exist in both the local config and in Portainer.

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
