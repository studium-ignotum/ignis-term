# Relay Server

The cloud relay that routes messages between Mac clients and browser clients.

## Overview

The relay server:
1. Accepts WebSocket connections from Mac clients (on `/mac`)
2. Accepts WebSocket connections from browsers (on `/browser`)
3. Generates session codes for pairing
4. Routes messages between paired clients

## Architecture

```
                    ┌─────────────────────────────────────┐
                    │           Relay Server              │
                    │                                     │
  Mac Client ──────►│ /mac endpoint                       │
                    │     │                               │
                    │     ▼                               │
                    │ ┌─────────────────┐                │
                    │ │ Session Registry │                │
                    │ │  - session codes  │               │
                    │ │  - mac→browser    │               │
                    │ │  - browser→mac    │               │
                    │ └─────────────────┘                │
                    │     │                               │
  Browser ─────────►│ /browser endpoint                   │
                    │                                     │
                    └─────────────────────────────────────┘
```

## Files

| File | Purpose |
|------|---------|
| `server.ts` | Main server, WebSocket handling, message routing |
| `session-registry.ts` | Session code generation, pairing logic, lifecycle |
| `shared/protocol.ts` | Zod message type definitions (shared with other components) |
| `shared/constants.ts` | Configuration values (ports, timeouts, etc.) |

## Running

```bash
pnpm start
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RELAY_PORT` | `8080` | WebSocket server port |

## Message Protocol

All messages are JSON with a `type` discriminator field, validated by Zod schemas.

### Mac → Relay

| Type | Description |
|------|-------------|
| `session_data` | Terminal output, tab updates |

### Browser → Relay

| Type | Description |
|------|-------------|
| `join` | Join session with code |
| `user_input` | Keyboard input for terminal |
| `select_tab` | Switch iTerm2 tab |

### Relay → Mac

| Type | Description |
|------|-------------|
| `registered` | Session code assigned |
| `browser_connected` | Browser joined session |
| `browser_disconnected` | Browser left session |
| `user_input` | Forward keyboard input |
| `select_tab` | Forward tab switch request |

### Relay → Browser

| Type | Description |
|------|-------------|
| `joined` | Successfully joined session |
| `session_list` | List of iTerm2 tabs |
| `terminal_data` | Terminal output to display |
| `tab_switch` | Tab switch confirmation |
| `error` | Error message |

## Session Codes

- 6 characters from `ABCDEFGHJKMNPQRSTVWXYZ23456789`
- Avoids lookalike characters (I/1, O/0, l)
- Expire after 5 minutes if unused
- Never expire once a browser connects
- Case-insensitive validation

## Connection Lifecycle

1. Mac connects to `/mac`
2. Server generates session code, sends `registered` message
3. Browser connects to `/browser`, sends `join` with code
4. Server validates code, pairs connections
5. Server sends `joined` to browser, `browser_connected` to Mac
6. Messages route bidirectionally
7. On disconnect, server notifies paired client

## Dependencies

- `ws` - WebSocket server
- `zod` - Schema validation
- `nanoid` - Session code generation
- `tsx` - TypeScript execution
