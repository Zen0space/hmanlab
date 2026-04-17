---
title: Quick Start
description: Run your first HmanLab agent in under a minute
---

This guide walks you through setting up HmanLab and running your first agent interaction.

## 1. Run the setup wizard

HmanLab includes an interactive onboarding command that configures your provider keys and workspace:

```bash
hmanlab onboard
```

This creates `~/.hmanlab/config.json` with your settings.

## 2. Or configure manually

Create the config directory and add your API key:

```bash
mkdir -p ~/.hmanlab

# Set your provider key
export HMANLAB_PROVIDERS_ANTHROPIC_API_KEY=sk-ant-...
# or
export HMANLAB_PROVIDERS_OPENAI_API_KEY=sk-...
```

## 3. Send your first message

```bash
hmanlab agent -m "Hello! What can you help me with?"
```

Add `--stream` for real-time token-by-token output:

```bash
hmanlab agent --stream -m "List the files in my current directory"
```

## 4. Use a template

HmanLab includes 4 built-in agent templates with specialized system prompts:

```bash
# Research mode
hmanlab agent --template researcher -m "What are the latest Rust async patterns?"

# Code assistant
hmanlab agent --template coder -m "Write a function to parse CSV files"

# List available templates
hmanlab template list
```

## 5. Validate your configuration

Check that everything is wired correctly:

```bash
hmanlab config check
```

This validates your config file and reports any issues.

## 6. Run in gateway mode

To serve your agent on Telegram, Slack, Discord, or Webhook:

```bash
# Set your channel token
export HMANLAB_CHANNELS_TELEGRAM_BOT_TOKEN=123456:ABC...

# Start the gateway
hmanlab gateway
```

## 7. Process prompts in batch

For bulk processing, create a text file with one prompt per line:

```bash
echo "Summarize the Rust ownership model" > prompts.txt
echo "Explain async/await in 3 sentences" >> prompts.txt

hmanlab batch --input prompts.txt --format jsonl
```

## What's next?

- Learn about the [agent loop](/docs/concepts/agent-loop/) to understand how messages are processed
- Browse available [tools](/docs/reference/tools/) your agent can use
- Set up [channels](/docs/concepts/channels/) for Telegram, Slack, or Discord
- Explore [plugins](/docs/guides/plugins/) to extend your agent with custom tools
