# stack-sync

Deploy and manage Docker Compose stacks from the command line. Supports two deployment modes:

- **Portainer mode** — Sync local Docker Compose files and environment variables to a [Portainer](https://www.portainer.io/) instance via its API
- **SSH mode** — Push stacks directly to remote hosts via SSH and `docker compose`, targeting [dockge](https://github.com/louislam/dockge)-style setups or any host with Docker Compose installed

## Prerequisites

### Portainer mode
- A Portainer instance with API access enabled
- A `PORTAINER_API_KEY` (create one in Portainer under **User Settings > Access Tokens**)

### SSH mode
- SSH access to the remote host (via key or agent)
- `docker compose` installed on the remote host

## Installation

### With Homebrew (tap)

```
brew install kyeotic/tap/stack-sync
```

### One-line Shell

```bash
curl -fsSL https://raw.githubusercontent.com/kyeotic/stack-sync/main/install | bash
```

### With Nix Flakes

```bash
nix profile install github:kyeotic/stack-sync
```

Or download a binary from the [releases page](https://github.com/kyeotic/stack-sync/releases).

## Quick Start

### Portainer Mode (default)

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

### SSH Mode

1. Create a `.stack-sync.toml` config file:

```toml
mode = "ssh"
host = "192.168.0.20"
ssh_user = "root"                # optional, defaults to current user
ssh_key = "~/.ssh/id_ed25519"    # optional, uses ssh-agent if omitted
host_dir = "/mnt/app_config/docker"

[stacks.my-stack]
compose_file = "compose.yaml"
env_file = ".env"  # optional
```

| Field      | Description                                       | Required |
| ---------- | ------------------------------------------------- | -------- |
| `mode`     | Set to `"ssh"` to enable SSH mode                 | Yes      |
| `host`     | SSH hostname or IP address                        | Yes      |
| `host_dir` | Remote directory where stacks are stored          | Yes      |
| `ssh_user` | SSH username                                      | No       |
| `ssh_key`  | Path to SSH private key (`~` is expanded)         | No       |

Stacks are deployed to `{host_dir}/{stack_name}/compose.yaml` on the remote host, with an optional `.env` file alongside it. This layout is compatible with [dockge](https://github.com/louislam/dockge) and similar tools.

SSH mode shells out to the `ssh` command on your system, so it inherits your SSH agent, `~/.ssh/config`, and `known_hosts` automatically.

### Deploy

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

Since creating a config file in your repo with secrets like the `portainer_api_key` is a bad practice, and since you're also likely to share the `host` with several projects, stack-sync supports config inheritance.

Global fields (`host`, `portainer_api_key`, `endpoint_id`, `mode`, `ssh_user`, `ssh_key`, `host_dir`) can be provided by a `.stack-sync.toml` in any parent directory up to and including your `$HOME` directory. This allows connection settings to be stored outside the working directory.

Alternatively you can provide a `PORTAINER_API_KEY` as an ENV VAR (e.g. sourced from a .env file by [dir-env](https://direnv.net/)) and not put it any config files.

The order of precedence is

- `$PORTAINER_API_KEY` ENV VAR
- current config (or config provided by `--config`)
- The next parent directory

Configs can also be merged: if the parent directory config contains an `endpoint_id` and the `$HOME` directory config contains a `host` they will form a complete configuration.

## Commands

### sync

Create or update stacks using the local compose files and env vars.

```bash
stack-sync sync                            # sync all stacks
stack-sync sync my-stack                   # sync one stack
stack-sync sync my-stack other-stack       # sync specific stacks
stack-sync sync my-stack --dry-run         # preview changes
stack-sync sync -C /path/to/config.toml    # use a different config file
```

The config path defaults to the current directory, where it will automatically look for `.stack-sync.toml` first, then `stack-sync.toml`. File paths in the config (`compose_file`, `env_file`) are resolved relative to the config file's directory, not the working directory.

### view

Show the current state of stacks on the remote.

```bash
stack-sync view                            # view all stacks
stack-sync view my-stack                   # view one stack
stack-sync view -C /path/to/config.toml    # use a different config file
```

### init

Initialize config files for a new project. Creates a parent config with credentials/connection settings and a local config with an example stack.

**Portainer mode** (default):

```bash
stack-sync init \
  --portainer-api-key ptr_xxx \
  --host https://portainer.example.com \
  --endpoint-id 2 \
  --parent-dir ~
```

**SSH mode:**

```bash
stack-sync init \
  --mode ssh \
  --host 192.168.0.20 \
  --host-dir /mnt/app_config/docker \
  --ssh-user root \
  --ssh-key ~/.ssh/id_ed25519 \
  --parent-dir ~
```

| Argument              | Description                                  | Required              |
| --------------------- | -------------------------------------------- | --------------------- |
| `--mode`              | Deploy mode: `portainer` or `ssh`            | No (default: portainer) |
| `--portainer-api-key` | Portainer API key                            | Portainer mode only   |
| `--host`              | Portainer URL or SSH hostname                | Yes                   |
| `--endpoint-id`       | Default endpoint ID (defaults to 2)          | No                    |
| `--host-dir`          | Remote directory for stacks                  | SSH mode only         |
| `--ssh-user`          | SSH username                                 | No                    |
| `--ssh-key`           | Path to SSH private key                      | No                    |
| `--parent-dir`        | Directory for credentials config (default ~) | No                    |
| `--force`             | Overwrite existing files                     | No                    |

This creates two files:
- `{parent-dir}/.stack-sync.toml` — credentials/connection settings
- `./.stack-sync.toml` — local config with example stack commented out

### import

Import an existing stack from Portainer (or a remote SSH host) into your local config. Downloads the compose file and env vars, and adds the stack to your config.

```bash
stack-sync import my-stack                    # import a stack
stack-sync import my-stack --force            # overwrite existing files
stack-sync import my-stack -C /path/to/dir    # use a different config directory
```

| Argument  | Description                                    | Required |
| --------- | ---------------------------------------------- | -------- |
| `<stack>` | Name of the stack to import                    | Yes      |
| `-C`      | Path to config file or directory               | No       |
| `--force` | Overwrite existing files                       | No       |

Creates `{stack}.compose.yaml` and `{stack}.env` files, and adds a `[stacks.{stack}]` entry to the local config. In SSH mode, the stack is read from `{host_dir}/{stack}/compose.yaml` on the remote host.

### redeploy

Force a stack to re-pull images and redeploy. Useful after pushing new images to a registry. In Portainer mode, this uses the current configuration from Portainer (not local files). In SSH mode, it runs `docker compose pull && docker compose up -d --force-recreate` on the remote host.

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

The stack must exist in both the local config and on the remote (Portainer or SSH host).

### upgrade

Check for the latest release and update the binary in-place.

```bash
stack-sync upgrade
```

> **Note:** Self-update is not supported when installed via Nix. Use `nix profile upgrade --flake github:kyeotic/stack-sync` instead.

## Env File Format

The `.env` file uses standard `KEY=value` format:

```
DATABASE_URL=postgres://localhost:5432/mydb
API_KEY=sk-abc123
DEBUG=true
```

Lines starting with `#` and blank lines are ignored.
