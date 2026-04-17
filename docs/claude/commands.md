# CLI Commands Reference

## Build & Run

```bash
cargo build --release
cargo build --release --features android    # Android device control
cargo build --release --features mqtt       # MQTT IoT channel

./target/release/hmanlab agent -m "Hello"
./target/release/hmanlab agent -m "Hello" --no-stream
./target/release/hmanlab agent --template <name> -m "..."
./target/release/hmanlab gateway
./target/release/hmanlab config check
./target/release/hmanlab provider status
```

## Interactive Slash Commands (inside `hmanlab agent`)

```
/help  /model  /model list  /model <provider:model>
/persona  /persona list  /persona <name>
/tools  /template  /history  /memory
/trust  /trust on  /trust off  /clear  /quit
```

Note: `/trust` and approval prompts only active when both stdin and stdout are real TTYs.

## Telegram Gateway Commands (in chat)

```
/model  /model list  /model reset  /model <provider:model>
/persona  /persona list  /persona <preset>  /persona <custom text>  /persona reset
```

## CLI Commands

```bash
# History
hmanlab history list [--limit 20]
hmanlab history show <query>
hmanlab history cleanup [--keep 50]

# Templates
hmanlab template list
hmanlab template show <name>

# Hands-lite
hmanlab hand list | activate <name> | deactivate | status

# Batch mode
hmanlab batch --input prompts.txt [--output results.jsonl --format jsonl --template coder --stop-on-error]

# Secrets
hmanlab secrets encrypt | decrypt | rotate

# Memory
hmanlab memory list [--category user]
hmanlab memory search "query"
hmanlab memory set <key> "value" --category user --tags "tag1,tag2"
hmanlab memory delete <key>
hmanlab memory stats

# Tools
hmanlab tools list
hmanlab tools info <name>

# Panel
hmanlab panel
hmanlab panel install | uninstall
hmanlab panel auth set-password | show-token

# Channels
hmanlab channel list | setup <name> | test <name>

# Quota
hmanlab quota status | reset [provider]

# Watch
hmanlab watch <url> --interval 1h --notify telegram

# Onboard
hmanlab onboard [--full]

# Update / Uninstall
hmanlab update [--check | --version v0.5.2 | --force]
hmanlab uninstall --yes [--remove-binary]

# Heartbeat & Skills
hmanlab heartbeat --show
hmanlab skills list

# Gateway with container/tunnel
hmanlab gateway --containerized [docker|apple]
hmanlab gateway --tunnel [cloudflare|ngrok|tailscale|auto]
```

## Release

```bash
# Requires: cargo install cargo-release
cargo release patch          # bug fixes (dry-run)
cargo release minor          # new functionality (dry-run)
cargo release patch --execute  # actually release

# patch = backward-compatible fixes, hardening, docs, internal refactors
# minor = new commands, flags, config fields, tools, providers, channels
```

## MCP Server Discovery

Config in `.mcp.json` or `~/.mcp/servers.json`:
```json
{"mcpServers":{"web":{"url":"http://localhost:3000"}}}
{"mcpServers":{"fs":{"command":"node","args":["server.js"]}}}
```
