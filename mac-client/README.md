# Mac Client

The Mac-side component that bridges iTerm2 to the relay server.

## Overview

This component runs on the user's Mac and:
1. Connects to the relay server via WebSocket
2. Receives a session code for browser pairing
3. Captures iTerm2 terminal output and sends it to connected browsers
4. Receives keyboard input from browsers and writes it to iTerm2

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│ Mac Client                                                       │
│                                                                 │
│  ┌──────────────┐    WebSocket    ┌────────────────────────┐   │
│  │ ConnectionMgr │ ◄────────────► │ Relay Server (remote) │    │
│  └──────────────┘                 └────────────────────────┘   │
│         │                                                       │
│         ▼                                                       │
│  ┌──────────────┐    Unix Socket   ┌─────────────────┐         │
│  │ SessionMgr   │ ◄──────────────► │ iterm-bridge.py │         │
│  └──────────────┘                  └─────────────────┘         │
│                                           │                     │
│                                           ▼                     │
│                                    ┌─────────────────┐         │
│                                    │ coprocess-      │         │
│                                    │ bridge.sh       │         │
│                                    │ (per session)   │         │
│                                    └─────────────────┘         │
│                                           │                     │
│                                           ▼                     │
│                                    ┌─────────────────┐         │
│                                    │ iTerm2 PTY      │         │
│                                    └─────────────────┘         │
└─────────────────────────────────────────────────────────────────┘
```

## Files

| File | Purpose |
|------|---------|
| `src/index.ts` | Entry point - creates managers and displays session code |
| `src/connection.ts` | WebSocket connection to relay with reconnection logic |
| `src/session-manager.ts` | Routes messages between relay and iTerm2 bridge |
| `src/iterm-bridge.ts` | Spawns and manages the Python bridge subprocess |
| `src/state-machine.ts` | Connection state validation |
| `iterm-bridge.py` | Python bridge using iTerm2 API (Unix socket server) |
| `coprocess-bridge.sh` | Bash script run as iTerm2 coprocess per terminal session |

## Running

```bash
# Development (with hot reload)
pnpm run dev

# Production
pnpm run build
pnpm start
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RELAY_URL` | `ws://localhost:8080/mac` | Relay server WebSocket URL |

## How It Works

### Terminal Output Flow

1. `coprocess-bridge.sh` runs as an iTerm2 coprocess for each terminal session
2. The coprocess captures stdout from the terminal
3. Data is sent via Unix socket to `iterm-bridge.py`
4. Python bridge Base64-encodes and wraps in JSON
5. `session-manager.ts` receives via Unix socket
6. Data is forwarded to relay via WebSocket

### User Input Flow

1. `session-manager.ts` receives `UserInput` message from relay
2. Message forwarded to Python bridge via Unix socket
3. Python bridge writes to the appropriate iTerm2 session
4. iTerm2 PTY receives and executes

### Tab Management

1. Python bridge queries iTerm2 for open tabs
2. Tab list sent to relay, forwarded to browser
3. When user selects a tab, `SelectTab` message is sent
4. Python bridge switches the active iTerm2 tab

## Dependencies

- `ws` - WebSocket client
- `tsx` - TypeScript execution
- Python 3.7+ with iTerm2 module
