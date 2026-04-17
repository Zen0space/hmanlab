---
title: Installation
description: Install HmanLab on your system
tableOfContents:
  minHeadingLevel: 2
  maxHeadingLevel: 4
---

HmanLab is distributed as a single static binary. Choose the installation method that works best for your platform.

## Prerequisites

- **macOS or Linux** — HmanLab runs on both platforms (x86_64 and ARM64)
- **No runtime dependencies** — The binary is fully self-contained

## Install with script (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/qhkm/hmanlab/main/install.sh | sh
```

This downloads the latest release binary for your platform and places it in your PATH.

## Install with Homebrew (macOS/Linux)

```bash
brew install qhkm/tap/hmanlab
```

## Install with Cargo

Build from source using Rust's package manager:

```bash
cargo install hmanlab --git https://github.com/qhkm/hmanlab
```

## Docker

Run HmanLab in a container:

```bash
docker pull ghcr.io/qhkm/hmanlab:latest

# Run agent mode
docker run --rm ghcr.io/qhkm/hmanlab:latest agent -m "Hello"

# Run gateway mode with config
docker run -d \
  -v ~/.hmanlab:/root/.hmanlab \
  -e HMANLAB_PROVIDERS_ANTHROPIC_API_KEY=sk-... \
  ghcr.io/qhkm/hmanlab:latest gateway
```

## Download binary

Pre-built binaries are available on the [releases page](https://github.com/qhkm/hmanlab/releases):

```bash
# Linux x86_64
curl -L https://github.com/qhkm/hmanlab/releases/latest/download/hmanlab-linux-x86_64 -o hmanlab
chmod +x hmanlab

# macOS (Apple Silicon)
curl -L https://github.com/qhkm/hmanlab/releases/latest/download/hmanlab-macos-aarch64 -o hmanlab
chmod +x hmanlab

# macOS (Intel)
curl -L https://github.com/qhkm/hmanlab/releases/latest/download/hmanlab-macos-x86_64 -o hmanlab
chmod +x hmanlab
```

## Build from source

To build from source, you need Rust 1.70+:

```bash
git clone https://github.com/qhkm/hmanlab.git
cd hmanlab

# Build release binary (~4MB)
cargo build --release

# Verify
./target/release/hmanlab --version
```

## Verify installation

```bash
hmanlab --version
# hmanlab 0.5.0

hmanlab --help
# Shows available commands
```

## Uninstall

Use the built-in uninstall command to remove HmanLab state:

```bash
hmanlab uninstall --yes

# Also remove a direct-install binary from ~/.local/bin or /usr/local/bin
hmanlab uninstall --remove-binary --yes
```

If you installed HmanLab with a package manager, remove the binary with the same tool:

```bash
brew uninstall qhkm/tap/hmanlab
cargo uninstall hmanlab
```

## Next steps

Now that HmanLab is installed, follow the [quick start guide](/docs/getting-started/quick-start/) to run your first agent interaction.
