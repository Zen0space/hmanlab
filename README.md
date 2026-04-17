<p align="center">
  <img src="assets/mascot-no-bg.png" width="200" alt="Zippy — HmanLab mascot">
</p>
<h1 align="center">HmanLab</h1>
<p align="center">
  <strong>Ultra-lightweight personal AI assistant.</strong>
</p>
<p align="center">
  <a href="https://hmanlab.com/docs/"><img src="https://img.shields.io/badge/docs-hmanlab.com-3b82f6?style=for-the-badge&logo=bookstack&logoColor=white" alt="Documentation"></a>
</p>
<p align="center">
  <a href="https://github.com/qhkm/hmanlab/actions/workflows/ci.yml"><img src="https://github.com/qhkm/hmanlab/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/qhkm/hmanlab/releases/latest"><img src="https://img.shields.io/github/v/release/qhkm/hmanlab?color=blue" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache%202.0-blue" alt="License"></a>
</p>

---

```
$ hmanlab agent --stream -m "Analyze our API for security issues"

🤖 HmanLab — Streaming analysis...

  [web_fetch]        Fetching API docs...
  [shell]            Running integration tests...
  [longterm_memory]  Storing findings...

→ Found 12 endpoints, 3 missing auth headers, 1 open redirect
→ Saved findings to long-term memory under "api-audit"

✓ Analysis complete in 4.2s
```

We studied the best AI assistants — and their tradeoffs. OpenClaw's integrations without the 100MB. NanoClaw's security without the TypeScript bundle. NemoClaw's governance without the 2GB Docker container. PicoClaw's size without the bare-bones feature set. One Rust binary with 33 tools, 11 channels, 16 providers, and 6 sandbox runtimes.

<p align="center">
  <img src="https://img.shields.io/badge/binary-~6MB-3b82f6" alt="~6MB binary">
  <img src="https://img.shields.io/badge/startup-~50ms-3b82f6" alt="~50ms startup">
  <img src="https://img.shields.io/badge/RAM-~6MB-3b82f6" alt="~6MB RAM">
  <img src="https://img.shields.io/badge/tests-3%2C900%2B-3b82f6" alt="3,900+ tests">
  <img src="https://img.shields.io/badge/providers-16-3b82f6" alt="16 providers">
</p>

## Why HmanLab

We studied what works — and what doesn't.

**OpenClaw** proved an AI assistant can handle 12 channels and 100+ skills. But it costs 100MB and 400K lines. **NanoClaw** proved security-first is possible. But it's still 50MB of TypeScript. **NemoClaw** proved enterprise governance matters — policy-locked sandboxes, federated inference routing. But it's a 2GB Docker container wrapping OpenClaw underneath, with zero built-in tools. **PicoClaw** proved AI assistants can run on $10 hardware. But it stripped out everything to get there.

**HmanLab** took notes. The integrations, the security, the governance, the size discipline — without the tradeoffs each one made. One 6MB Rust binary that starts in 50ms, uses 6MB of RAM, and ships with container isolation, prompt injection detection, and a circuit breaker provider stack.

| | OpenClaw | NemoClaw | NanoClaw | PicoClaw | **HmanLab** |
|---|---|---|---|---|---|
| **Size** | ~100MB | ~2GB (Docker) | ~50MB | <1MB | **~6MB** |
| **Language** | JS/TS | JS/TS/Python | TypeScript | Go | **Rust** |
| **Built-in tools** | 100+ skills | 0 (inference only) | ~20 | ~5 | **33** |
| **Providers** | 5 | NVIDIA-first | 3 | 2 | **16** |
| **Channels** | 12 | 0 (uses OpenClaw) | 3 | 0 | **11** |
| **Sandbox** | None | Landlock + seccomp | Basic | None | **6 runtimes** |
| **Runs on $10 HW** | No | No (needs GPU) | No | Yes | **Yes** |

## Security

AI agents execute code. Most frameworks trust that nothing will go wrong.

The OpenClaw ecosystem has seen CVE-2026-25253 (CVSS 8.8 — cross-site WebSocket hijacking to RCE), ClawHavoc (341 malicious skills, 9,000+ compromised installations), and 42,000 exposed instances with auth bypass. HmanLab was built with this threat model in mind.

| Layer | What it does |
|-------|-------------|
| **6 Sandbox Runtimes** | Docker, Apple Container, Landlock, Firejail, Bubblewrap, or native — per request |
| **Prompt Injection Detection** | Aho-Corasick multi-pattern matcher (17 patterns) + 4 regex rules |
| **Secret Leak Scanner** | 22 regex patterns catch API keys, tokens, and credentials before they reach the LLM |
| **Policy Engine** | 7 rules blocking system file access, crypto key extraction, SQL injection, encoded exploits |
| **Input Validator** | 100KB limit, null byte detection, whitespace ratio analysis, repetition detection |
| **Shell Blocklist** | Regex patterns blocking reverse shells, `rm -rf`, privilege escalation |
| **SSRF Prevention** | DNS pinning, private IP blocking, IPv6 transition guard, scheme validation |
| **Chain Alerting** | Detects dangerous tool call sequences (write→execute, memory→execute) |
| **Tool Approval Gate** | Require explicit confirmation before executing dangerous tools |

Every layer runs by default. No flags to remember, no config to enable.

## Install

```bash
# One-liner (macOS / Linux)
curl -fsSL https://raw.githubusercontent.com/qhkm/hmanlab/main/install.sh | sh

# Homebrew
brew install qhkm/tap/hmanlab

# Docker
docker pull ghcr.io/qhkm/hmanlab:latest

# Build from source
cargo install hmanlab --git https://github.com/qhkm/hmanlab
```

The control panel is an optional compile-time feature. To use `hmanlab panel` or
`hmanlab serve`, build/install with `--features panel`.

## Uninstall

```bash
# Remove HmanLab state (~/.hmanlab)
hmanlab uninstall --yes

# Also remove a direct-install binary from ~/.local/bin or /usr/local/bin
hmanlab uninstall --remove-binary --yes

# Package-managed installs still use their package manager
brew uninstall qhkm/tap/hmanlab
cargo uninstall hmanlab
```

## Quick Start

```bash
# Interactive setup (walks you through API keys, channels, workspace)
hmanlab onboard

# Talk to your agent
hmanlab agent -m "Hello, set up my workspace"

# Stream responses token-by-token
hmanlab agent --stream -m "Explain async Rust"

# Use a built-in template
hmanlab agent --template researcher -m "Search for Rust agent frameworks"

# Process prompts in batch
hmanlab batch --input prompts.txt --output results.jsonl

# Start as a Telegram/Slack/Discord/Webhook gateway
hmanlab gateway

# With full container isolation per request
hmanlab gateway --containerized
```

## Migrate from OpenClaw

Already running OpenClaw? HmanLab can import your config and skills in one command.

```bash
# Auto-detect OpenClaw installation (~/.openclaw, ~/.clawdbot, ~/.moldbot)
hmanlab migrate

# Specify path manually
hmanlab migrate --from /path/to/openclaw

# Preview what would be migrated (no files written)
hmanlab migrate --dry-run

# Non-interactive (skip confirmation prompts)
hmanlab migrate --yes
```

The migration command:
- Converts provider API keys, model settings, and channel configs
- Copies skills to `~/.hmanlab/skills/`
- Backs up your existing HmanLab config before overwriting
- Validates the migrated config and reports any issues
- Lists features that can't be automatically ported

Supports JSON and JSON5 config files (comments, trailing commas, unquoted keys).

## Deploy

<p align="center">
  <a href="https://cloud.digitalocean.com/apps/new?repo=https://github.com/qhkm/hmanlab/tree/main"><img src="https://img.shields.io/badge/DigitalOcean-0080FF?style=for-the-badge&logo=digitalocean&logoColor=white" alt="Deploy to DigitalOcean"></a>
  <a href="https://railway.com/deploy?template=https://github.com/qhkm/hmanlab"><img src="https://img.shields.io/badge/Railway-0B0D0E?style=for-the-badge&logo=railway&logoColor=white" alt="Deploy to Railway"></a>
  <a href="https://render.com/deploy?repo=https://github.com/qhkm/hmanlab"><img src="https://img.shields.io/badge/Render-46E3B7?style=for-the-badge&logo=render&logoColor=white" alt="Deploy to Render"></a>
  <a href="https://fly.io/docs/hands-on/"><img src="https://img.shields.io/badge/Fly.io-6E42C1?style=for-the-badge&logo=fly.io&logoColor=white" alt="Deploy to Fly.io"></a>
</p>

### Any VPS

```bash
curl -fsSL https://hmanlab.com/setup.sh | bash
```

Installs the binary and prints next steps. Run `hmanlab onboard` to configure providers and channels.

## Providers

HmanLab supports 16 LLM providers. All OpenAI-compatible endpoints work out of the box.

| Provider | Config key | Setup |
|----------|------------|-------|
| **Anthropic** | `anthropic` | `api_key` |
| **OpenAI** | `openai` | `api_key` |
| **OpenRouter** | `openrouter` | `api_key` |
| **Google Gemini** | `gemini` | `api_key` |
| **Groq** | `groq` | `api_key` |
| **DeepSeek** | `deepseek` | `api_key` |
| **xAI (Grok)** | `xai` | `api_key` |
| **NVIDIA NIM** | `nvidia` | `api_key` |
| **Azure OpenAI** | `azure` | `api_key` + `api_base` |
| **AWS Bedrock** | `bedrock` | `api_key` |
| **Kimi (Moonshot)** | `kimi` | `api_key` |
| **Zhipu (GLM)** | `zhipu` | `api_key` |
| **Qianfan (Baidu)** | `qianfan` | `api_key` |
| **Novita AI** | `novita` | `api_key` |
| **Ollama** | `ollama` | `api_key` (any value) |
| **VLLM** | `vllm` | `api_key` (any value) |

Configure in `~/.hmanlab/config.json` or via environment variables:

```json
{
  "providers": {
    "openrouter": { "api_key": "sk-or-..." },
    "ollama": { "api_key": "ollama" }
  },
  "agents": { "defaults": { "model": "anthropic/claude-sonnet-4" } }
}
```

```bash
export HMANLAB_PROVIDERS_GROQ_API_KEY=gsk_...
```

Any provider's base URL can be overridden with `api_base` for proxies or self-hosted endpoints. See the [provider docs](https://hmanlab.com/docs/concepts/providers/) for full details.

## Features

### Core

| Feature | What it does |
|---------|-------------|
| **Multi-Provider LLM** | 16 providers with SSE streaming, retry with backoff + budget cap, auto-failover |
| **33 Tools + Plugins** | Shell, filesystem, grep, find, web, git, stripe, PDF, transcription, Android ADB, and more |
| **Tool Composition** | Create new tools from natural language descriptions — composable `{{param}}` templates |
| **Agent Swarms** | Delegate to sub-agents with parallel fan-out, aggregation, and cost-aware routing |
| **Library Facade** | Embed as a crate — `HmanLabAgent::builder().provider(p).tool(t).build()` for Tauri/GUI apps |
| **Batch Mode** | Process hundreds of prompts from text/JSONL files with template support |
| **Agent Modes** | Observer, Assistant, Autonomous — category-based tool access control |

### Channels & Integration

| Feature | What it does |
|---------|-------------|
| **11-Channel Gateway** | Telegram, Slack, Discord, WhatsApp Web + Cloud API, Lark, Email, Webhook, Serial, ACP — unified message bus |
| **Persona System** | Per-chat personality switching via `/persona` command with LTM persistence |
| **Plugin System** | JSON manifest plugins auto-discovered from `~/.hmanlab/plugins/` |
| **Hooks** | `before_tool`, `after_tool`, `on_error` with Log, Block, and Notify actions |
| **Cron & Heartbeat** | Schedule recurring tasks, proactive check-ins, background spawning |
| **Memory & History** | Workspace memory, long-term key-value store, conversation history |

### Security & Ops

| Feature | What it does |
|---------|-------------|
| **6 Sandbox Runtimes** | Docker, Apple Container, Landlock, Firejail, Bubblewrap, or native |
| **Gateway Startup Guard** | Degrade gracefully after N crashes — prevents crash loops |
| **Channel Supervisor** | Auto-restart dead channels with cooldown and max-restart limits |
| **Tool Approval Gate** | Policy-based gating — require confirmation for dangerous tools |
| **SSRF Prevention** | DNS pinning, private IP blocking, IPv6 transition guard, scheme validation |
| **Shell Blocklist** | Regex patterns blocking reverse shells, rm -rf, privilege escalation |
| **Token Budget & Cost** | Per-session budget enforcement, per-model cost estimation for 8 models |
| **Rich Health Endpoint** | `/health` with version, uptime, RSS, usage metrics, component checks |
| **Telemetry** | Prometheus + JSON metrics export, structured logging, per-tenant tracing |
| **Self-Update** | `hmanlab update` downloads latest release from GitHub |
| **Loop Guard** | SHA256 tool-call repetition detection with circuit-breaker stop |
| **Context Trimming** | Normal/emergency/critical compaction tiers (70%/90%/95%) for context window management |
| **Session Repair** | Auto-fixes orphan tool results, empty/duplicate messages, and alternation issues |
| **Config Hot-Reload** | Gateway polls config mtime every 30s and applies provider/channel/safety updates live |
| **Hands-Lite** | `HAND.toml` agent profiles with bundled presets (researcher, coder, monitor) and `hand` CLI |
| **Multi-Tenant** | Hundreds of tenants on one VPS — isolated workspaces, ~6MB RAM each |

> **Full documentation** — [hmanlab.com/docs](https://hmanlab.com/docs/) covers configuration, environment variables, CLI reference, deployment guides, and more.

## Inspired By

HmanLab is inspired by projects in the open-source AI agent ecosystem — OpenClaw, NemoClaw, NanoClaw, and PicoClaw — each taking a different approach to the same problem. NemoClaw's declarative policy model and digest-verified supply chain influenced our security thinking. HmanLab's contribution is Rust's memory safety, async performance, and container isolation for production multi-tenant deployments — all in a 6MB binary that runs where Docker containers can't.

## Usage

```bash
# CLI agent (one-shot or streaming)
hmanlab agent -m "Summarize this repo"
hmanlab agent --stream -m "Explain async Rust"
hmanlab agent --template coder -m "Add error handling to main.rs"

# Multi-channel gateway
hmanlab gateway                    # Telegram, Slack, Discord, etc.
hmanlab gateway --containerized    # With container isolation per request

# Memory, secrets, profiles
hmanlab memory set project:name "HmanLab" --category project
hmanlab secrets encrypt
hmanlab hand activate researcher

# Batch, diagnostics, self-update
hmanlab batch --input prompts.txt --output results.jsonl
hmanlab doctor                     # Diagnose config/provider issues
hmanlab update                     # Self-update to latest release
```

## Development

```bash
# Build
cargo build

# Run all tests (~3,900 total)
cargo nextest run --lib

# Lint and format (required before every PR)
cargo clippy -- -D warnings
cargo fmt -- --check
```

See [CLAUDE.md](CLAUDE.md) for full architecture reference, [AGENTS.md](AGENTS.md) for coding guidelines, and [docs/](docs/) for benchmarks, multi-tenant deployment, and performance guides.

## HmanLab Stack

HmanLab is part of the HmanLab stack — a modular system for running AI agents in production.

```
HmanLabPM       — orchestration, supervision, retries, job lifecycle
    │
    │  create(spec) + spawn(worker, args, env)
    ▼
HmanLabCapsule  — capsule creation, process isolation, resource enforcement
    │
    │  fork/namespace/microVM + stdio transport
    ▼
HmanLab       — LLM calls, tool use, artifact production
    │
    └── JSON-line IPC over stdin/stdout back to HmanLabPM
```

| Layer | Repo | Role |
|:------|:-----|:-----|
| **HmanLabPM** | [qhkm/zeptopm](https://github.com/qhkm/zeptopm) | Process manager — config-driven daemon, HTTP API, pipelines, orchestration |
| **HmanLabCapsule** | [qhkm/zeptocapsule](https://github.com/qhkm/zeptocapsule) | Sandbox — process/namespace/Firecracker isolation, resource limits, fallback chains |
| **HmanLabRT** | [qhkm/zeptort](https://github.com/qhkm/zeptort) | Durable runtime — journaled effects, snapshot recovery, OTP-style supervision |
| **HmanLab** | [qhkm/hmanlab](https://github.com/qhkm/hmanlab) | Agent framework — 33 tools, 16 providers, 11 channels, container isolation |

## Contributing

We welcome contributions! Please read **[CONTRIBUTING.md](CONTRIBUTING.md)** for:

- How to set up your fork and branch from upstream
- Issue-first workflow (open an issue before coding)
- Pull request process and quality gates
- Guides for adding new tools, channels, and providers

## License

Apache 2.0 — see [LICENSE](LICENSE)

## Disclaimer

HmanLab is a pure open-source software project. It has no token, no cryptocurrency, no blockchain component, and no financial instrument of any kind. This project is not affiliated with any token or financial product.

---

<p align="center">
  <em>HmanLab — Because your AI assistant shouldn't need more RAM than your text editor.</em>
</p>
<p align="center">
  Built by <a href="https://aisar.ai">Aisar Labs</a>
</p>

---

For commercial licensing, enterprise support, or managed hosting inquiries: **qaiyyum@aisar.ai**
