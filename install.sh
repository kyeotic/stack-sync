#!/bin/bash
# Wrapper installer for stack-sync.
# Usage: curl -fsSL https://raw.githubusercontent.com/kyeotic/stack-sync/main/install.sh | bash
set -euo pipefail

export INSTALL_REPO="kyeotic/stack-sync"
export INSTALL_BINARY="stack-sync"

# Download and run the generic installer
curl -fsSL "https://raw.githubusercontent.com/kyeotic/pipe-install/refs/heads/main/rust" | bash
