---
phase: 02-terminal-iterm2
plan: 03
subsystem: mac-client-integration
tags: [iterm2-bridge, session-manager, unix-socket, ipc, subprocess, terminal-io]
depends_on:
  requires: [02-01, 02-02]
  provides: [mac-client-bridge-integration, terminal-io-routing, session-management]
  affects: [02-04, 02-05]
tech_stack:
  added: []
  patterns: [subprocess-ipc, event-bridge, protocol-translation, auto-restart]
key_files:
  created:
    - mac-client/src/iterm-bridge.ts
    - mac-client/src/session-manager.ts
  modified:
    - mac-client/src/connection.ts
    - mac-client/src/index.ts
decisions:
  - id: "fileURLToPath-for-dirname"
    decision: "Use fileURLToPath(import.meta.url) instead of import.meta.dirname for ESM path resolution"
    rationale: "moduleResolution 'bundler' in tsconfig does not support import.meta.dirname; fileURLToPath is universally supported in ESM Node.js"
metrics:
  duration: "3 min"
  completed: "2026-02-05"
---

# Phase 2 Plan 3: Mac Client Node.js Integration Layer Summary

Node.js integration layer connecting iTerm2 Python bridge to WebSocket relay via subprocess management, Unix socket IPC, and bidirectional protocol translation for terminal I/O routing.

## What Was Done

### Task 1: Create iterm-bridge.ts (3193c41)

Created `mac-client/src/iterm-bridge.ts` (284 lines) -- the Node.js manager for the Python bridge subprocess and Unix socket IPC.

**Subprocess management:**
- Spawns `python3 iterm-bridge.py <socket-path>` via `child_process.spawn`
- Captures stdout/stderr for logging, detects missing `iterm2` pip package
- Auto-restarts crashed subprocess after 3-second delay
- Cancels pending restart timer on graceful stop

**Unix socket IPC:**
- Connects to Python bridge's Unix domain socket with retry (10 attempts, 500ms delay)
- Parses JSON lines protocol (newline-delimited JSON objects)
- Handles partial messages with line buffering

**Event interface:**
- `terminal_data` -- base64-decoded terminal output from iTerm2
- `sessions` -- session list with tab IDs, titles, active state
- `tab_switched` -- tab focus changes from iTerm2
- `tabs_changed` -- layout changes (tab created/closed)
- `config` -- iTerm2 profile configuration
- `ready` -- bridge initialization complete
- `error` / `exit` -- error and process lifecycle events

**Command methods:**
- `sendInput(sessionId, data)` -- forward keyboard input (base64 encoded)
- `sendResize(sessionId, cols, rows)` -- terminal resize
- `switchTab(tabId)`, `createTab()`, `closeTab(tabId)` -- tab operations

### Task 2: Create session-manager.ts and update index.ts (cde7af4)

**session-manager.ts (173 lines):**
- Translates iTerm2 bridge IPC events to WebSocket protocol messages:
  - `terminal_data` (base64 Buffer) -> `{type: 'terminal_data', sessionId, payload}`
  - `sessions` / `tabs_changed` -> `{type: 'tab_list', tabs: [{tabId, sessionId, title, isActive}]}`
  - `tab_switched` -> `{type: 'tab_switch', tabId}`
  - `config` -> `{type: 'config', font, fontSize, cursorStyle, cursorBlink, scrollback, theme}`
- Maps iTerm2 cursor types (CURSOR_TYPE_BLOCK/UNDERLINE/VERTICAL) to protocol values (block/underline/bar)
- Parses iTerm2 font format ("FontName Size") into separate font + fontSize fields
- Maps 16 ANSI colors to named theme keys
- Handles incoming relay messages: terminal_input, terminal_resize, tab_switch, tab_create, tab_close

**connection.ts changes:**
- Added `onMessage?: (data: string) => void` callback to ConnectionEvents
- Default message handler case forwards unhandled types via onMessage (previously logged warning)
- Added `send(data: string): void` method for outbound messages

**index.ts changes:**
- Creates SessionManager with sendToRelay callback wired to `manager.send()`
- Wires ConnectionManager onMessage to `sessionManager.handleRelayMessage()`
- Starts iTerm2 bridge on `connected` state change
- Graceful shutdown: stops bridge before disconnecting WebSocket
- Error handlers stop bridge before exit

## Data Flow

```
iTerm2 Session
  |  (coprocess PTY I/O)
  v
coprocess-bridge.sh
  |  (Unix socket)
  v
iterm-bridge.py (Python)
  |  (Unix socket, JSON lines)
  v
iterm-bridge.ts (Node.js)
  |  (EventEmitter)
  v
session-manager.ts
  |  (JSON string)
  v
connection.ts -> WebSocket -> relay -> browser
```

Input flows in reverse via `handleRelayMessage()`.

## Decisions Made

| Decision | Rationale | Impact |
|----------|-----------|--------|
| fileURLToPath for ESM dirname | tsconfig moduleResolution "bundler" lacks import.meta.dirname types | Used fileURLToPath(import.meta.url) + path.dirname for Python script path |
| Start bridge on 'connected' state | Bridge needs relay connection to forward data | sessionManager.start() called in onStateChange connected handler |
| Async shutdown with bridge stop first | Bridge subprocess must be killed before WebSocket disconnects | shutdown() awaits sessionManager.stop() then calls manager.disconnect() |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Used fileURLToPath instead of import.meta.dirname**
- **Found during:** Task 1 compilation
- **Issue:** Plan used `import.meta.dirname` but tsconfig moduleResolution "bundler" does not provide types for this Node.js-specific property
- **Fix:** Used `fileURLToPath(import.meta.url)` + `path.dirname()` which is universally supported in ESM
- **Files modified:** mac-client/src/iterm-bridge.ts
- **Commit:** 3193c41

## Verification Results

- TypeScript strict compilation: PASS (all 5 source files, zero errors)
- ITerm2Bridge exports all 8 methods: PASS (start, stop, send, sendInput, sendResize, switchTab, createTab, closeTab)
- SessionManager imports and uses ITerm2Bridge: PASS (13 references)
- Protocol message types used: PASS (terminal_data, tab_list, tab_switch, config, terminal_input, terminal_resize)
- ConnectionManager send() method: PASS
- ConnectionManager onMessage callback: PASS
- index.ts creates and wires SessionManager: PASS
- Key links patterns matched: PASS (all 4 patterns)

## Next Phase Readiness

All terminal I/O routing infrastructure is in place for:
- **02-04**: Browser terminal (xterm.js) can receive terminal_data and config messages
- **02-05**: Tab management UI can receive tab_list and tab_switch messages, send tab operations
- Bridge auto-restart ensures resilience for long-running sessions

---
*Phase: 02-terminal-iterm2*
*Completed: 2026-02-05*
