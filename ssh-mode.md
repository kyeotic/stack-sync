# SSH-Mode

Currently stack-sync only supports portainer deploys. I want to add a new deployment mode: SSH-Mode. This mode will take the following configuration

```toml
mode="ssh"
host="192.168.0.20"
ssh_key="~/.ssh/id_ed25519"
host_dir="/mnt/app_config/docke"
```

This will be used to push stacks into directories on the host. 

- Starting stacks will be done with `docker compose up`
- Stopping stacks (e.g. `enabled=false`) will be done with `docker compose down`

When deploying a stack

- create a directory inside the `host_dir` named after the stack
- create a `compose.yaml` containing the compose configuration in the stack dir
- create a `.env` file containing the env configuration in the stack dir, if env is defined

This mode is primarily designed to work with dockge, but it should be fairly general since dockge aims to just use the filesystem and normal docker compose as the engine.