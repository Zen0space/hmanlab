# Changelog

All notable changes to HmanLab will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.6.1] - 2026-02-27

### Added
- **Control panel web UI** — axum API server with REST routes, WebSocket event streaming, JWT auth, CSRF protection; React + Vite + Tailwind frontend with Dashboard, Logs, Sessions, Cron/Routines, Kanban (drag-and-drop), and live Agent Office pages; `hmanlab panel` CLI command (#180)
- **Google Workspace tools** — Gmail search/read/send/reply + Calendar list/create/freebusy via gogcli-rs; feature-gated behind `--features google` (#162)

### Fixed
- **Per-message memory injection** — LLM now receives query-matched relevant memories (not just pinned) in every message, including tool-loop follow-up calls (#188)
- Shared single LTM instance between agent injection and `longterm_memory` tool (eliminates duplicate initialization)
- Per-skill install from multi-skill repos (#169)
- Per-provider model mapping for fallback chains (#166)
- Added `version` spec to `gog-*` git dependencies for crates.io publishing compatibility

### Changed
- Test count: 2,612 lib tests

## [0.6.0] - 2026-02-26

### Added
- **Loop guard** — SHA256 tool-call repetition detection with configurable window and threshold; warns then circuit-breaks on repeated patterns (#171)
- **Emergency context trimming** — Three-tier compaction (normal 70% / emergency 90% / critical 95%) with reduced tool result budgets at higher tiers (#172)
- **Session repair** — Auto-fixes orphan tool results, empty/duplicate messages, and role alternation issues before sending to LLM (#173)
- **Config hot-reload** — Gateway polls config file mtime every 30s; applies provider, channel, and safety changes without restart (#174)
- **Hands-lite** — `HAND.toml` agent profiles with bundled presets (researcher, coder, monitor); `hand list/activate/deactivate/status` CLI commands (#176)
- **Telegram forum topic support** — Messages in forum topics include `thread_id` for thread-aware replies (#170)

### Fixed
- RPi I2C block read/write method names corrected for rppal API (`block_read`/`block_write`)
- Tool result budget now uses configured `max_tool_result_bytes` instead of hardcoded 5120 in emergency compaction (#177)

### Changed
- Dependency upgrades: teloxide 0.12→0.17, tokio-tungstenite 0.21→0.28
- Test count increased from 2,880+ to 2,900+

## [0.5.9] - 2026-02-26

### Added
- **HmanLabAgent facade improvements** — Sequential execution, callback support, and repair semantics for embedding as a crate (#157)

### Fixed
- Upgrade teloxide 0.12→0.17 and tokio-tungstenite 0.21→0.28 for compatibility (#156)

## [0.5.8] - 2026-02-25

### Added
- **HmanLabAgent library facade** — `HmanLabAgent::builder().provider(p).tool(t).build()` for embedding HmanLab as a crate in Tauri/Electron/GUI apps; persistent conversation history behind `Mutex` for thread-safe concurrent access (#154)
- **Parallel fan-out for multi-agent aggregate** — DelegateTool now supports parallel dispatch to multiple sub-agents with result aggregation (#150)
- **RPi peripheral parity with ESP32** — Raspberry Pi GPIO + native I2C tools via rppal with board profile pin validation (#152)
- **Gateway startup guard** — Degrade gracefully after N crashes to prevent crash loops; configurable crash threshold (#147)
- **DOM-based web extraction** — `web_fetch` now uses `scraper` crate for proper HTML→text extraction instead of regex stripping (#146)
- **Rich health endpoint** — `/health` returns version, uptime, memory RSS, usage metrics, component checks; `/ready` returns boolean readiness (#145)
- **Hardware features in release binaries** — CI release builds now bake in `peripheral-esp32` and `peripheral-rpi` features (#144)
- **ESP32 hardware tools** — GPIO read/write, I2C scan/read/write, NVS get/set/delete, Serial channel for UART messaging (#139)
- **Natural language tool composition** — `CreateToolTool` with create/list/delete/run actions; `ComposedTool` interpolates `{{param}}` placeholders into action templates (#138)
- **Soul/persona system** — Per-chat personality switching via `/persona` command (list, set preset, custom text, reset) with long-term memory persistence (#133)
- **Channel supervisor** — Polling supervisor (15s) detects dead channels via `is_running()`, restarts with 60s cooldown, max 5 restarts, reports to HealthRegistry (#117)
- **Self-update command** — `hmanlab update` downloads latest release from GitHub; supports `--check`, `--version`, `--force` flags (#111)
- **Linux sandbox runtimes** — Landlock LSM (kernel 5.13+), Firejail (namespace + seccomp), Bubblewrap (OCI-compatible `bwrap`) — feature-gated (#104)
- **Shell allowlist mode** — Off/Warn/Strict modes for command allowlisting alongside existing blocklist
- **Retry budget** — `retry_budget_ms` wall-clock cap on total retry time (default: 45s) (#135)
- **Board profiles** — Pin ranges and capability registry per board type (ESP32, RPi, Arduino, Nucleo)

### Changed
- Channel count increased from 8 to 9 (added Serial/UART channel)
- Runtime count increased from 3 to 6 (added Landlock, Firejail, Bubblewrap)
- Test count increased from 2,300+ to 2,880+
- Web fetch uses DOM-based extraction (scraper) instead of regex HTML stripping

### Fixed
- Telegram allowlist silently broken since v0.5.0 due to dptree type collision (#109)
- SSRF guard bypass via IPv6-to-IPv4 transition addresses (#105)
- Doctor false negative on binary detection in containers (#134)
- Stale state on task exit in cron, heartbeat, lark, gateway (#118)
- OpenAI `max_tokens` → `max_completion_tokens` retry for known model families
- Filesystem writer tool serialization to prevent race conditions (#113)
- Full GitHub URLs in skills install (#131)

### Security
- Memory poisoning guard — injection pattern detection on `longterm_memory.set()` (#124)
- Tiered inbound injection scanning — block webhooks, warn-only on allowlisted channels (#124)
- Tool chain alerting — detects dangerous sequences (write→execute, execute→fetch, memory→execute) (#124)
- SSRF IPv6-to-IPv4 transition address blocking
- Shell allowlist mode (Off/Warn/Strict) alongside existing blocklist
- Config path blocklist prevents LLM-driven config exfiltration

## [0.5.0] - 2026-02-22

### Added
- **Android device control** — Feature-gated ADB tool (`--features android`) with screen perception via uiautomator XML parsing, 22 actions (tap, type, swipe, scroll, launch, screenshot, etc.), stuck detection, and URL scheme validation
- **Voice transcription** — WhatsApp Cloud voice message transcription with configurable provider support
- **Telegram /model command** — Runtime LLM switching from chat (`/model list`, `/model <provider:model>`, `/model reset`) with per-chat persistence
- **Agent modes** — Category-based autonomy levels (Observer, Assistant, Autonomous) replacing numeric autonomy levels
- **Response cache** — LLM response caching to reduce duplicate API calls
- **Device pairing** — USB device discovery and pairing support for hardware integrations
- **Hardware tool** — GPIO, serial, and USB peripheral operations
- **HTTP request tool** — General-purpose HTTP client tool for arbitrary API calls
- **PDF read tool** — Extract text content from PDF files
- **Transcribe tool** — Audio transcription with provider abstraction
- **Git tool** — Git operations (status, diff, log, commit) as an agent tool
- **Project tool** — Project scaffolding and management operations
- **Stripe tool** — Stripe API integration for payment operations with production hardening
- **Skills search & install** — `find_skills` and `install_skill` tools for runtime skill discovery
- **Web screenshot tool** — Capture webpage screenshots
- **Skill registry** — Centralized skill discovery and management
- **Provider plugins** — External LLM provider support via plugin system
- **Error classifier** — Structured provider error classification for smarter retry/fallback
- **Provider cooldown** — Rate-limit-aware cooldown periods between provider requests
- **Structured logging** — Configurable log levels and format via `utils/logging.rs`
- **Lark channel** — Lark/Feishu messaging integration
- **Email channel** — Email-based agent interaction
- **WhatsApp Cloud channel** — Official WhatsApp Cloud API (webhook + REST, no bridge dependency)
- **Claude Code subscription auth** — OAuth token support for Anthropic providers
- **Smarter retry** — Improved retry logic with error classification and backoff tuning
- **Gemini native provider** — Direct Google Gemini API support
- **Pluggable memory backends** — BM25, embedding, HNSW, Tantivy searcher options
- **Agent swarm improvements** — Parallel dispatch, aggregation, scratchpad, cost-aware routing
- **Production polish** — Sandbox mode, heartbeat delivery, extensibility improvements
- **Onboard OpenRouter** — OpenRouter added to provider setup menu
- **R8r tool enhancements** — Status, emit, and create actions

### Changed
- Tool count increased from 18 to 29 built-in tools (+ android feature-gated)
- Channel count increased from 5 to 8 (added Lark, Email, WhatsApp Cloud)
- Test count increased from 1,560 to 2,300+
- Autonomy levels renamed to agent modes (category-based)
- Dockerfile Rust version updated to 1.93

### Fixed
- UTF-8 truncation panic in web.rs and custom.rs
- RISC-V getrandom SIGSEGV via build.rs cfg override
- Broken interactive prompts in setup.sh
- Cross-PR commit contamination detection in CI

### Security
- Android tool URL scheme allowlist (blocks javascript:, file:, intent:)
- Android tool busybox/toybox shell command bypass prevention
- Android tool shell metacharacter blocking
- Audit logging for security events
- WhatsApp sender authentication
- Plugin SHA256 verification
- Apple Container gating

## [0.4.0] - 2026-02-15

### Added
- **Secret encryption at rest** — XChaCha20-Poly1305 AEAD with Argon2id KDF; `ENC[version:salt:nonce:ciphertext]` format stored in config.json; `secrets encrypt/decrypt/rotate` CLI commands; transparent decryption on config load
- **Tunnel support** — Cloudflare, ngrok, and Tailscale tunnel providers; `--tunnel` gateway flag with auto-detect mode; subprocess lifecycle management
- **Deny-by-default sender allowlists** — `deny_by_default` bool on all channel configs; when true + empty allowlist = reject all messages
- **Memory decay and injection** — Importance-weighted decay scoring for long-term memory; pinned memories auto-injected into system prompt; pre-compaction memory flush
- **Memory pin action** — `pin` action on longterm_memory tool for always-included context
- **OpenAI-compatible provider tests** — 13 tests confirming `api_base` works for Ollama, Groq, Together, Fireworks, LM Studio, vLLM
- **OpenClaw migration** — `hmanlab migrate` command to import config and skills from OpenClaw installations
- **Binary plugin system** — JSON-RPC 2.0 stdin/stdout protocol for external tool binaries
- **Reminder tool** — Persistent reminder store with 6 actions; task-manager agent template
- **Custom tools** — CLI-defined tools via `custom_tools` config with compact descriptions
- **Tool profiles** — Named tool subsets for different agent configurations
- **Agent engine resilience** — Structured provider errors, three-tier overflow recovery, circuit breaker on fallback, dynamic tool result budgets, runtime context injection
- **URL watch command** — `hmanlab watch <url>` monitors pages for changes with channel notifications
- **Tool discovery CLI** — `hmanlab tools list` and `hmanlab tools info <name>`
- **Memory CLI** — `hmanlab memory list/search/set/delete/stats`
- **Express onboard** — Streamlined setup as default, full wizard behind `--full` flag
- **CLI smoke tests** — Integration test suite for CLI command validation
- **OG meta tags** — Open Graph and Twitter Card meta for landing page

### Changed
- Rebrand positioning to "A complete AI agent runtime in 4MB"
- Tool count increased from 17 to 18 built-in tools

### Security
- Prompt injection detection (17 patterns + 4 regex via Aho-Corasick)
- Secret leak scanning (22 regex patterns)
- Security policy engine (7 rules)
- Input validation (length, null bytes, repetition detection)
- XChaCha20-Poly1305 secret encryption with OWASP-recommended Argon2id params (m=64MB, t=3, p=1)
- Deny-by-default sender allowlists propagated to all channel spawned tasks

## [0.2.0] - 2026-02-14

First public release.

### Added
- **Streaming responses** — Token-by-token SSE streaming for Claude and OpenAI providers (`--stream` flag)
- **Agent swarms** — DelegateTool creates specialist sub-agents with role-specific system prompts and tool whitelists
- **Plugin system** — JSON manifest-based plugin discovery and registration with PluginTool adapter
- **Agent templates** — Pre-configured agent profiles (coder, researcher, etc.) with `--template` flag
- **4 channels** — Telegram, Slack (outbound), Discord (Gateway WebSocket + REST), Webhook (HTTP POST inbound)
- **Batch mode** — Process multiple prompts from text/JSONL files with `batch` CLI command
- **Conversation history** — CLI commands to list, search, and clean up past sessions
- **Long-term memory** — Persistent key-value store with categories, tags, and keyword search
- **Token budget** — Per-session token budget tracking with atomic counters
- **Structured output** — JSON and JSON Schema output format support for OpenAI and Claude
- **Tool approval** — Configurable approval gate checked before tool execution
- **Retry provider** — Exponential backoff wrapper for 429/5xx errors
- **Fallback provider** — Automatic primary-to-secondary provider failover
- **Cost tracking** — Per-provider/model cost accumulation with pricing tables for 8 models
- **Telemetry export** — Prometheus text exposition and JSON metrics rendering
- **Hooks system** — Config-driven before_tool, after_tool, on_error hooks with pattern matching
- **17 built-in tools** — shell, filesystem (read/write/list/edit), web search, web fetch, memory, cron, spawn, delegate, WhatsApp, Google Sheets, message, long-term memory, r8r
- **Container isolation** — Native, Docker, and Apple Container runtimes
- **Multi-tenant deployment** — Per-tenant isolation with Docker Compose templates
- **Cross-platform CI/CD** — GitHub Actions for test/lint/fmt, cross-platform release builds (4 targets), Docker image push

### Security
- Shell command blocklist with regex patterns
- Path traversal protection with symlink escape detection
- SSRF prevention with DNS pre-resolution against private IPs
- Workspace-scoped filesystem tools
- Mount allowlist validation
- Cron job caps and spawn recursion prevention

[0.6.0]: https://github.com/Zen0space/hmanlab/releases/tag/v0.6.0
[0.5.9]: https://github.com/Zen0space/hmanlab/releases/tag/v0.5.9
[0.5.8]: https://github.com/Zen0space/hmanlab/releases/tag/v0.5.8
[0.5.0]: https://github.com/Zen0space/hmanlab/releases/tag/v0.5.0
[0.4.0]: https://github.com/Zen0space/hmanlab/releases/tag/v0.4.0
[0.2.0]: https://github.com/Zen0space/hmanlab/releases/tag/v0.2.0
