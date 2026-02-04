# Architecture Patterns

**Domain:** Remote Terminal Control (Web-based iTerm2 remote access)
**Researched:** 2026-02-04
**Confidence:** MEDIUM (based on training data - external verification unavailable)

## System Overview

Remote terminal control systems follow a three-tier relay architecture:

```
+------------------+       +------------------+       +------------------+
|   Mac Client     | <---> |   Cloud Relay    | <---> |    Browser       |
| (iTerm2 control) |  WSS  | (session router) |  WSS  | (xterm.js UI)    |
+------------------+       +------------------+       +------------------+
        |
        v
  +------------+
  |  iTerm2    |
  |  (local)   |
  +------------+
```

## Component Boundaries

### Component 1: Mac Client (iTerm2 Bridge)

**Responsibility:** Interface between iTerm2 and the cloud relay

| Subcomponent | Purpose |
|-------------|---------|
| iTerm2 API Client | Connects to iTerm2's Python API socket |
| Session Manager | Tracks active tabs/windows/sessions |
| Input Forwarder | Receives keystrokes from relay, sends to iTerm2 |
| Output Streamer | Captures terminal output, forwards to relay |
| Relay Connection | Maintains persistent WebSocket to cloud |

**Communicates With:**
- iTerm2 (local Unix socket via Python API)
- Cloud Relay (outbound WebSocket)

**Key Technical Details:**
- iTerm2 exposes a Python API via `iterm2` library
- Can enumerate windows, tabs, sessions
- Can subscribe to session output via `async for`
- Can send text/keystrokes to sessions
- Supports coprocess and custom escape sequences

**Confidence:** MEDIUM - iTerm2 Python API is well-documented, but specific capabilities should be verified against current docs

### Component 2: Cloud Relay (Session Router)

**Responsibility:** Route messages between Mac clients and browsers, manage session authentication

| Subcomponent | Purpose |
|-------------|---------|
| WebSocket Server | Accept connections from both clients and browsers |
| Session Registry | Map session codes to client connections |
| Message Router | Forward messages bidirectionally |
| Auth Handler | Validate session codes, manage permissions |
| Heartbeat Manager | Detect disconnections, cleanup stale sessions |

**Communicates With:**
- Mac Client (inbound WebSocket from client)
- Browser (inbound WebSocket from browser)

**Key Design Decisions:**

1. **Session Code Model:** Short-lived codes (like "ABC-123") that browsers use to connect
2. **Stateless Routing:** Relay doesn't interpret terminal data, just forwards bytes
3. **Connection Lifecycle:** Client registers, gets code, browser joins with code

**Confidence:** HIGH - Standard WebSocket relay patterns

### Component 3: Browser (Terminal UI)

**Responsibility:** Render terminal, capture input, display session selector

| Subcomponent | Purpose |
|-------------|---------|
| xterm.js Terminal | Render terminal output, capture keystrokes |
| Session Selector UI | Show available tabs, allow switching |
| Relay Connection | Maintain WebSocket to cloud relay |
| Fit Addon | Auto-resize terminal to container |
| Connection Status | Show connected/disconnected state |

**Communicates With:**
- Cloud Relay (outbound WebSocket)
- User (keyboard/mouse input, visual output)

**Key Technical Details:**
- xterm.js is the standard web terminal emulator (used by VS Code, etc.)
- Handles ANSI escape sequences, colors, cursor movement
- Has addon system for fit, search, weblinks
- `terminal.write()` for output, `terminal.onData()` for input

**Confidence:** HIGH - xterm.js is well-established

## Data Flow

### Flow 1: Terminal Output (iTerm2 to Browser)

```
iTerm2 Session
    |
    | (Python API: async for output)
    v
Mac Client: Output Streamer
    |
    | WebSocket message: {type: "output", sessionId, data}
    v
Cloud Relay: Message Router
    |
    | Forward to connected browser(s)
    v
Browser: Relay Connection
    |
    | terminal.write(data)
    v
xterm.js Terminal (renders)
```

**Message Format:**
```json
{
  "type": "output",
  "sessionId": "session-uuid",
  "data": "\u001b[32mHello World\u001b[0m\n"
}
```

### Flow 2: Keyboard Input (Browser to iTerm2)

```
User types keystroke
    |
    | terminal.onData(callback)
    v
Browser: Input Handler
    |
    | WebSocket message: {type: "input", sessionId, data}
    v
Cloud Relay: Message Router
    |
    | Forward to Mac client
    v
Mac Client: Input Forwarder
    |
    | session.async_send_text(data)
    v
iTerm2 Session (receives input)
```

**Message Format:**
```json
{
  "type": "input",
  "sessionId": "session-uuid",
  "data": "ls -la\r"
}
```

### Flow 3: Session Discovery (List Available Tabs)

```
Browser requests session list
    |
    v
Cloud Relay: Forward request to Mac client
    |
    v
Mac Client: Session Manager
    |
    | Enumerate via iTerm2 API:
    | - app.windows
    | - window.tabs
    | - tab.sessions
    v
Mac Client: Send session list
    |
    v
Cloud Relay: Forward to browser
    |
    v
Browser: Render session selector
```

**Session List Message:**
```json
{
  "type": "sessions",
  "sessions": [
    {
      "id": "session-uuid-1",
      "name": "zsh",
      "windowTitle": "Terminal",
      "tabTitle": "zsh",
      "isActive": true
    },
    {
      "id": "session-uuid-2",
      "name": "vim",
      "windowTitle": "Terminal",
      "tabTitle": "vim file.txt",
      "isActive": false
    }
  ]
}
```

### Flow 4: Session Switch

```
User clicks different tab in browser
    |
    v
Browser: Send switch request
    |
    | {type: "switch", sessionId: "new-session-uuid"}
    v
Cloud Relay: Update routing, forward
    |
    v
Mac Client: Update active session
    |
    | Start streaming output from new session
    | Optionally: send current screen contents
    v
Browser: Receives new session output
```

### Flow 5: Connection Establishment

```
1. Mac Client starts
   |
   v
2. Client connects to Relay
   |
   | {type: "register", clientId: "..."}
   v
3. Relay generates session code
   |
   | Response: {type: "registered", code: "ABC-123"}
   v
4. Client displays code to user
   |
   v
5. User enters code in browser
   |
   v
6. Browser connects to Relay with code
   |
   | {type: "join", code: "ABC-123"}
   v
7. Relay links browser to client
   |
   | Response: {type: "joined", sessions: [...]}
   v
8. Bidirectional streaming begins
```

## Recommended Architecture

### Protocol Design

Use a simple JSON message protocol over WebSocket:

```typescript
// Message types
type Message =
  | { type: "register"; clientId: string }
  | { type: "registered"; code: string }
  | { type: "join"; code: string }
  | { type: "joined"; sessions: Session[] }
  | { type: "output"; sessionId: string; data: string }
  | { type: "input"; sessionId: string; data: string }
  | { type: "switch"; sessionId: string }
  | { type: "sessions"; sessions: Session[] }
  | { type: "resize"; cols: number; rows: number }
  | { type: "ping" }
  | { type: "pong" }
  | { type: "error"; message: string };
```

### State Management

**Relay State (minimal):**
```
sessions: Map<code, {
  clientConnection: WebSocket,
  browserConnections: Set<WebSocket>,
  createdAt: Date
}>
```

**Client State:**
```
- Connected relay WebSocket
- Active iTerm2 app connection
- Map of session IDs to iTerm2 session objects
- Current streaming session
```

**Browser State:**
```
- Connected relay WebSocket
- xterm.js Terminal instance
- Available sessions list
- Current session ID
- Connection status
```

## Patterns to Follow

### Pattern 1: Message Framing with Type Discrimination

**What:** Use a `type` field to discriminate message kinds
**When:** All WebSocket communication
**Why:** Enables type-safe handling, easy to extend

```typescript
// Good
{ "type": "output", "sessionId": "...", "data": "..." }

// Avoid - no type discrimination
{ "output": "...", "session": "..." }
```

### Pattern 2: Heartbeat for Connection Health

**What:** Regular ping/pong to detect dead connections
**When:** All WebSocket connections
**Why:** WebSocket doesn't detect dead connections quickly

```typescript
// Every 30 seconds
client.send({ type: "ping" });
// Expect pong within 5 seconds or mark disconnected
```

### Pattern 3: Reconnection with Backoff

**What:** Auto-reconnect with exponential backoff
**When:** Client or browser loses connection
**Why:** Network interruptions are common

```typescript
const reconnect = (attempt: number) => {
  const delay = Math.min(1000 * Math.pow(2, attempt), 30000);
  setTimeout(() => connect(), delay);
};
```

### Pattern 4: Binary vs Text Data

**What:** Use text WebSocket frames for terminal data
**When:** Terminal I/O
**Why:** Terminal data is text-based (with ANSI escapes)

Note: If supporting file transfers later, use binary frames for that.

### Pattern 5: Lazy Session Streaming

**What:** Only stream output from the active session
**When:** Multiple sessions available
**Why:** Reduces bandwidth and processing

```python
# Only subscribe to output from the active session
async with session.output_listener() as listener:
    async for notification in listener:
        send_to_relay(notification.content)
```

## Anti-Patterns to Avoid

### Anti-Pattern 1: Buffering All Output on Relay

**What:** Storing terminal history in the relay
**Why bad:**
- Memory grows unbounded
- Terminal scrollback can be huge
- Not the relay's responsibility
**Instead:** Let xterm.js handle scrollback locally. For new browser connections, request screen refresh from client.

### Anti-Pattern 2: Polling for Sessions

**What:** Browser repeatedly asking for session list
**Why bad:** Wastes bandwidth, slow updates
**Instead:** Push session changes from client when tabs open/close

### Anti-Pattern 3: Direct Client IP Exposure

**What:** Browser connecting directly to Mac client
**Why bad:** Requires port forwarding, exposes home IP
**Instead:** Always route through cloud relay

### Anti-Pattern 4: Synchronous iTerm2 API Calls

**What:** Blocking while waiting for iTerm2 response
**Why bad:** iTerm2 API is async, blocking kills performance
**Instead:** Use async/await throughout Python client

### Anti-Pattern 5: Unbounded Session Codes

**What:** Session codes that never expire
**Why bad:** Security risk, stale connections
**Instead:** Codes expire after connection or timeout (e.g., 5 minutes unused)

## Component Dependencies (Build Order)

Based on architecture analysis, recommended build order:

```
Phase 1: Foundation
├── Message protocol definition (shared types)
├── Basic WebSocket relay (accepts connections)
└── Minimal browser terminal (xterm.js renders)

Phase 2: Mac Client
├── iTerm2 API connection
├── Session enumeration
├── Output streaming from one session
└── Input forwarding to one session

Phase 3: Full Integration
├── Session code auth flow
├── Session switching
├── Multiple browser support
└── Reconnection handling

Phase 4: Polish
├── Terminal resize handling
├── Connection status UI
├── Error recovery
└── Performance optimization
```

**Rationale:**
1. **Relay first:** Both client and browser depend on it
2. **Browser basic:** Faster iteration testing with mock data
3. **Mac client:** Complex iTerm2 integration
4. **Integration:** Connect all pieces
5. **Polish:** Reliability and UX

## Scalability Considerations

| Concern | 1 User | 10 Users | 100 Users |
|---------|--------|----------|-----------|
| Relay Memory | Minimal | ~50MB | Consider Redis for session registry |
| Latency | <50ms | <50ms | <100ms (add regional relays) |
| Auth | Simple codes | Simple codes | Consider accounts + persistent codes |

For initial product, single relay instance handles hundreds of concurrent sessions easily.

## Security Considerations

| Concern | Mitigation |
|---------|------------|
| Session code guessing | Use 6+ character codes, rate limit attempts |
| Man-in-middle | WSS (TLS) required for all connections |
| Unauthorized access | Codes expire, single-use after join |
| Terminal injection | No server-side interpretation of terminal data |
| Client impersonation | Client generates unique ID, relay validates |

## iTerm2 API Reference

The Mac client relies heavily on iTerm2's Python API:

```python
import iterm2

async def main(connection):
    app = await iterm2.async_get_app(connection)

    # Get all windows
    for window in app.windows:
        for tab in window.tabs:
            for session in tab.sessions:
                # session.session_id - unique identifier
                # session.name - shell name
                # await session.async_send_text("hello")
                # await session.async_get_screen_contents()
                pass

    # Subscribe to output
    async with session.output_listener() as listener:
        async for notification in listener:
            # notification.content contains new output
            pass

iterm2.run_until_complete(main)
```

**Key API Methods:**
- `iterm2.async_get_app()` - Get app object
- `app.windows` - List of windows
- `window.tabs` - List of tabs in window
- `tab.sessions` - List of sessions in tab
- `session.async_send_text(str)` - Send input
- `session.async_get_screen_contents()` - Get current screen
- `session.output_listener()` - Subscribe to output stream

**Confidence:** MEDIUM - API surface is correct but should verify exact method signatures against current iTerm2 Python API docs

## Sources

- xterm.js architecture based on established patterns (VS Code terminal, many web IDEs)
- iTerm2 Python API based on official documentation patterns
- WebSocket relay patterns from common real-time architecture practices
- **Note:** External source verification was unavailable; confidence is MEDIUM based on training data

## Verification Needed

Before implementation, verify:
1. [ ] Current iTerm2 Python API method signatures
2. [ ] xterm.js v5+ API changes (addons, lifecycle)
3. [ ] WebSocket library choices for Node.js relay (ws, uWebSockets)
4. [ ] SvelteKit WebSocket integration patterns
