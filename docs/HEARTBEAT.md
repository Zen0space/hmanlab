# Heartbeat Service

Heartbeat periodically asks HmanLab to review background tasks from `HEARTBEAT.md`.

## Config

```json
{
  "heartbeat": {
    "enabled": true,
    "interval_secs": 1800,
    "file_path": "~/.hmanlab/HEARTBEAT.md"
  }
}
```

## CLI

```bash
hmanlab heartbeat --show
hmanlab heartbeat --edit
hmanlab heartbeat
```

## How It Works

1. Heartbeat reads the configured heartbeat file on each interval.
2. If the file has actionable content, it sends a heartbeat prompt to the agent.
3. The agent reads `HEARTBEAT.md` in workspace context and executes tasks.

Use comments (`<!-- ... -->`) and headers for notes that should not trigger work.
