#!/bin/bash
set -euo pipefail

case "$(uname -s)" in
    Darwin) os="apple-darwin" ;;
    Linux)  os="unknown-linux-musl" ;;
    *)      echo "Unsupported OS"; exit 1 ;;
esac

case "$(uname -m)" in
    x86_64|amd64)  arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
    *)             echo "Unsupported architecture: $(uname -m)"; exit 1 ;;
esac

if [ "$os" = "unknown-linux-musl" ] && [ "$arch" != "x86_64" ]; then
    echo "Unsupported Linux architecture: $arch"
    exit 1
fi

target="${arch}-${os}"
repo="kyeotic/stack-sync"

echo "Fetching latest release..."
tag=$(curl -fsSL "https://api.github.com/repos/${repo}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)

if [ -z "$tag" ]; then
    echo "Failed to fetch latest release tag"
    exit 1
fi

echo "Downloading stack-sync ${tag} for ${target}..."
url="https://github.com/${repo}/releases/download/${tag}/stack-sync-${target}.tar.gz"
curl -fsSL "$url" | tar xz

echo "Installing to /usr/local/bin/stack-sync..."
sudo mv stack-sync /usr/local/bin/stack-sync
echo "stack-sync ${tag} installed successfully"
