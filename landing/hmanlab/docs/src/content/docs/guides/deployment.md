---
title: Deployment
description: Deploy HmanLab to production
tableOfContents:
  minHeadingLevel: 2
  maxHeadingLevel: 3
---

HmanLab can be deployed anywhere a Linux binary can run. Choose the method that fits your infrastructure.

## One-click deploy

### Any VPS

The fastest way to deploy HmanLab is using the automated setup script:

```bash
curl -fsSL https://hmanlab.com/setup.sh | bash
```

This interactive wizard will:
- Download the latest HmanLab binary
- Guide you through configuring your LLM provider (Anthropic or OpenAI)
- Set up your messaging channel (Telegram, Slack, Discord, or Webhook)
- Install and start a systemd service

**Docker deployment:**
```bash
curl -fsSL https://hmanlab.com/setup.sh | bash -s -- --docker
```

**Uninstall:**
```bash
curl -fsSL https://hmanlab.com/setup.sh | bash -s -- --uninstall
```

## Docker (single container)

The simplest production deployment:

```dockerfile
FROM ghcr.io/qhkm/hmanlab:latest

COPY config.json /root/.hmanlab/config.json

EXPOSE 8080
CMD ["hmanlab", "gateway"]
```

```bash
docker build -t my-agent .
docker run -d --name hmanlab \
  -e HMANLAB_PROVIDERS_ANTHROPIC_API_KEY=sk-ant-... \
  -e HMANLAB_CHANNELS_TELEGRAM_BOT_TOKEN=123456:ABC... \
  my-agent
```

## Docker Compose

For multi-service setups:

```yaml
version: '3.8'
services:
  hmanlab:
    image: ghcr.io/qhkm/hmanlab:latest
    restart: unless-stopped
    environment:
      - HMANLAB_PROVIDERS_ANTHROPIC_API_KEY=${ANTHROPIC_KEY}
      - HMANLAB_CHANNELS_TELEGRAM_BOT_TOKEN=${TELEGRAM_TOKEN}
    volumes:
      - hmanlab-data:/root/.hmanlab
    healthcheck:
      test: ["CMD", "hmanlab", "config", "check"]
      interval: 30s
      retries: 3

volumes:
  hmanlab-data:
```

## Fly.io

Deploy to Fly.io with a single command:

```toml
# fly.toml
app = "my-hmanlab"
primary_region = "sin"

[build]
  image = "ghcr.io/qhkm/hmanlab:latest"

[env]
  RUST_LOG = "info"

[[services]]
  internal_port = 8080
  protocol = "tcp"

  [[services.ports]]
    port = 443
    handlers = ["tls", "http"]
```

```bash
fly launch
fly secrets set HMANLAB_PROVIDERS_ANTHROPIC_API_KEY=sk-ant-...
fly secrets set HMANLAB_CHANNELS_TELEGRAM_BOT_TOKEN=123456:ABC...
fly deploy
```

## Railway

Deploy via Railway CLI:

```bash
railway init
railway up

# Set secrets
railway variables set HMANLAB_PROVIDERS_ANTHROPIC_API_KEY=sk-ant-...
railway variables set HMANLAB_CHANNELS_TELEGRAM_BOT_TOKEN=123456:ABC...
```

## Render

Use the Render dashboard or `render.yaml`:

```yaml
services:
  - type: worker
    name: hmanlab
    runtime: docker
    dockerfilePath: ./Dockerfile
    envVars:
      - key: HMANLAB_PROVIDERS_ANTHROPIC_API_KEY
        sync: false
      - key: HMANLAB_CHANNELS_TELEGRAM_BOT_TOKEN
        sync: false
```

## Systemd (bare metal)

For direct deployment on a Linux server:

```ini
# /etc/systemd/system/hmanlab.service
[Unit]
Description=HmanLab AI Agent
After=network-online.target

[Service]
Type=simple
User=hmanlab
ExecStart=/usr/local/bin/hmanlab gateway
Restart=always
RestartSec=5
Environment=RUST_LOG=info
EnvironmentFile=/etc/hmanlab/env

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable hmanlab
sudo systemctl start hmanlab
```

## Health checks

HmanLab's `config check` command can be used as a health check:

```bash
hmanlab config check
```

Returns exit code 0 if configuration is valid.

## Persistent data

Important directories to persist across restarts:

| Path | Contents |
|------|----------|
| `~/.hmanlab/config.json` | Configuration |
| `~/.hmanlab/memory/` | Long-term memory |
| `~/.hmanlab/sessions/` | Conversation history |
| `~/.hmanlab/skills/` | Custom skills |
| `~/.hmanlab/plugins/` | Custom plugins |

Mount `~/.hmanlab` as a volume in Docker deployments.

## Resource requirements

HmanLab is very lightweight:

| Resource | Requirement |
|----------|-------------|
| CPU | Any modern CPU (ARM64 or x86_64) |
| RAM | ~6MB RSS at idle |
| Disk | ~4MB binary + data |
| Network | Outbound HTTPS to LLM provider APIs |
