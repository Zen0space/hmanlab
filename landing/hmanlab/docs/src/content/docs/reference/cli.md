---
title: CLI Reference
description: Complete command reference for the HmanLab CLI
tableOfContents:
  minHeadingLevel: 2
  maxHeadingLevel: 3
---

HmanLab uses a subcommand-based CLI built with [clap](https://docs.rs/clap).

## Global options

```
hmanlab [OPTIONS] <COMMAND>
```

| Option | Description |
|--------|-------------|
| `--help` | Show help message |
| `--version` | Show version |

## agent

Run a single agent interaction.

```bash
hmanlab agent [OPTIONS] -m <MESSAGE>
```

| Option | Description |
|--------|-------------|
| `-m, --message <TEXT>` | Message to send to the agent |
| `--stream` | Enable streaming (token-by-token output) |
| `--template <NAME>` | Use an agent template (coder, researcher, writer, analyst) |
| `--workspace <PATH>` | Set workspace directory |

### Examples

```bash
# Simple message
hmanlab agent -m "Hello"

# With streaming
hmanlab agent --stream -m "Explain async Rust"

# With template
hmanlab agent --template coder -m "Write a CSV parser"
```

## gateway

Start the multi-channel gateway.

```bash
hmanlab gateway [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--containerized [RUNTIME]` | Enable container isolation (auto, docker, apple) |
| `--tunnel [PROVIDER]` | Enable tunnel (auto, cloudflare, ngrok, tailscale) |

### Examples

```bash
# Start gateway
hmanlab gateway

# With container isolation
hmanlab gateway --containerized docker
```

## batch

Process multiple prompts from a file.

```bash
hmanlab batch [OPTIONS] --input <FILE>
```

| Option | Description |
|--------|-------------|
| `--input <FILE>` | Input file (text or JSONL) |
| `--output <FILE>` | Output file (default: stdout) |
| `--format <FORMAT>` | Output format: text, jsonl |
| `--template <NAME>` | Agent template to use |
| `--stream` | Enable streaming per prompt |
| `--stop-on-error` | Stop on first error |

### Examples

```bash
# Process text file
hmanlab batch --input prompts.txt

# JSONL output
hmanlab batch --input prompts.txt --format jsonl --output results.jsonl

# With template and error handling
hmanlab batch --input prompts.jsonl --template researcher --stop-on-error
```

## config check

Validate configuration file.

```bash
hmanlab config check
```

Reports unknown fields, missing required values, and type errors.

## history

Manage conversation history.

```bash
hmanlab history <SUBCOMMAND>
```

### history list

```bash
hmanlab history list [--limit <N>]
```

List recent sessions with timestamps and titles.

### history show

```bash
hmanlab history show <QUERY>
```

Show a session by fuzzy-matching the query against session titles and keys.

### history cleanup

```bash
hmanlab history cleanup [--keep <N>]
```

Remove old sessions, keeping the most recent N (default: 50).

## template

Manage agent templates.

```bash
hmanlab template <SUBCOMMAND>
```

### template list

List all available templates (built-in and custom).

### template show

```bash
hmanlab template show <NAME>
```

Show template details including system prompt, model, and overrides.

## onboard

Run the interactive setup wizard.

```bash
hmanlab onboard [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--full` | Run the full 10-step wizard instead of express setup |

Walks through provider key setup, channel configuration, and workspace initialization.

## heartbeat

View heartbeat service status.

```bash
hmanlab heartbeat --show
```

## uninstall

Remove HmanLab state and optionally the current binary.

```bash
hmanlab uninstall [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--remove-binary` | Remove the current binary for direct installs in `~/.local/bin` or `/usr/local/bin` |
| `-y, --yes` | Skip the confirmation prompt |

### Examples

```bash
# Remove ~/.hmanlab
hmanlab uninstall --yes

# Remove ~/.hmanlab and a direct-install binary
hmanlab uninstall --remove-binary --yes
```

## skills

Manage agent skills.

```bash
hmanlab skills list
```

List available skills from `~/.hmanlab/skills/`.

## secrets

Manage secret encryption at rest.

```bash
hmanlab secrets <SUBCOMMAND>
```

### secrets encrypt

Encrypt plaintext API keys and tokens in your config file using XChaCha20-Poly1305.

```bash
hmanlab secrets encrypt
```

### secrets decrypt

Decrypt secrets for editing.

```bash
hmanlab secrets decrypt
```

### secrets rotate

Re-encrypt with a new master key.

```bash
hmanlab secrets rotate
```

## memory

Manage long-term memory from the CLI.

```bash
hmanlab memory <SUBCOMMAND>
```

### memory list

```bash
hmanlab memory list [--category <CATEGORY>]
```

### memory search

```bash
hmanlab memory search <QUERY>
```

### memory set

```bash
hmanlab memory set <KEY> <VALUE> [--category <CATEGORY>] [--tags <TAGS>]
```

### memory delete

```bash
hmanlab memory delete <KEY>
```

### memory stats

```bash
hmanlab memory stats
```

## tools

Discover available tools.

```bash
hmanlab tools <SUBCOMMAND>
```

### tools list

List all available tools and their status.

```bash
hmanlab tools list
```

### tools info

Show detailed info about a specific tool.

```bash
hmanlab tools info <NAME>
```

## watch

Monitor a URL for changes.

```bash
hmanlab watch <URL> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--interval <DURATION>` | Check interval (e.g., 1h, 30m) |
| `--notify <CHANNEL>` | Channel for notifications |

## channel

Manage channels.

```bash
hmanlab channel <SUBCOMMAND>
```

### channel list

```bash
hmanlab channel list
```

### channel setup

```bash
hmanlab channel setup <NAME>
```

### channel test

```bash
hmanlab channel test <NAME>
```

## migrate

Import config and skills from an OpenClaw installation.

```bash
hmanlab migrate [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--from <PATH>` | Path to OpenClaw installation |
| `--dry-run` | Preview migration without writing files |
| `--yes` | Non-interactive (skip prompts) |
