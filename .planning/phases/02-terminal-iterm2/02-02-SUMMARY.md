---
phase: 02-terminal-iterm2
plan: 02
subsystem: iterm2-bridge
tags: [iterm2, python-api, coprocess, pty, unix-socket, json-lines, base64]

# Dependency graph
requires:
  - phase: 02-terminal-iterm2
    provides: "Extended protocol schemas for terminal data and tab management (02-01)"
provides:
  - "iTerm2 Python API bridge for session discovery, coprocess management, tab monitoring, config reading"
  - "Coprocess shell script bridging PTY I/O to Unix domain socket"
  - "JSON lines IPC protocol between Python bridge and Node.js Mac client"
affects: [02-03-mac-client-integration, 02-04-browser-terminal, 02-05-tab-management]

# Tech tracking
tech-stack:
  added: [iterm2-python-api]
  patterns: [coprocess-per-session, json-lines-ipc, base64-terminal-data, unix-domain-socket-ipc]

key-files:
  created:
    - mac-client/iterm-bridge.py
    - mac-client/coprocess-bridge.sh
    - mac-client/requirements.txt
  modified: []

key-decisions:
  - "JSON lines over Unix domain socket for Python-to-Node.js IPC"
  - "One coprocess socket per session (avoids multiplexing complexity)"
  - "Base64 encoding for terminal data (raw PTY bytes may be non-UTF-8)"
  - "socat preferred with nc fallback for coprocess-to-socket bridge"
  - "atexit handler for socket file cleanup"

patterns-established:
  - "Coprocess-per-session: each iTerm2 session gets its own coprocess and dedicated Unix socket"
  - "JSON lines IPC: one JSON object per line, newline-delimited, over Unix domain socket"
  - "Base64 terminal data: raw PTY bytes encoded as base64 in JSON messages"
  - "Monitor-based event model: FocusMonitor, LayoutChangeMonitor, NewSessionMonitor for real-time updates"

# Metrics
duration: 3min
completed: 2026-02-04
---

# Phase 2 Plan 2: iTerm2 Python Bridge + Coprocess Summary

**iTerm2 Python API bridge with coprocess-per-session PTY capture, real-time tab/focus monitoring, profile config reading, and JSON lines IPC over Unix domain sockets**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-04T19:44:17Z
- **Completed:** 2026-02-04T19:47:19Z
- **Tasks:** 2
- **Files created:** 3

## Accomplishments
- Python bridge discovers all existing iTerm2 sessions on startup and attaches coprocesses for raw PTY byte capture
- Real-time monitoring of tab focus changes, layout changes, and new sessions via iTerm2 FocusMonitor/LayoutChangeMonitor/NewSessionMonitor
- Profile configuration reading (font, colors, cursor style, scrollback) sent to Node.js client
- Bidirectional command handling: terminal input, tab switch/create/close from browser
- Coprocess bridge shell script connects PTY stdin/stdout to Unix socket with socat (nc fallback)
- Robust error handling, socket cleanup, connection retry logic

## Task Commits

Each task was committed atomically:

1. **Task 1: Create iTerm2 Python bridge with session management and config reading** - `e0a27f0` (feat)
2. **Task 2: Create coprocess bridge shell script** - `09e4f40` (feat)

## Files Created/Modified
- `mac-client/iterm-bridge.py` - iTerm2 Python API bridge: session discovery, coprocess management, tab monitoring, config reading, command handling (358 lines)
- `mac-client/coprocess-bridge.sh` - Shell script run as iTerm2 coprocess, bridges PTY I/O to Unix socket via socat/nc (103 lines)
- `mac-client/requirements.txt` - Python dependencies: iterm2>=2.7

## Decisions Made
- **JSON lines over Unix domain socket for IPC**: Simple, debuggable protocol; one JSON object per newline. Unix sockets chosen over TCP for low latency and no port conflicts.
- **One coprocess socket per session**: Each iTerm2 session gets a dedicated Unix socket for its coprocess data, avoiding multiplexing complexity in the shell script.
- **Base64 encoding for terminal data**: Raw PTY bytes may contain non-UTF-8 sequences, so base64 ensures safe JSON transport.
- **socat preferred with nc fallback**: socat handles bidirectional I/O cleanly; nc + named pipe fallback works on stock macOS without brew.
- **atexit + signal handler for cleanup**: Socket files cleaned up reliably on normal exit, SIGTERM, and exceptions.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- socat is not installed on this system (stock macOS). The coprocess script includes a nc fallback, and `brew install socat` is recommended for production use. Not a blocker.

## User Setup Required

None - no external service configuration required. Note: iTerm2 must have its Python API enabled (Preferences > General > Magic > Enable Python API) for the bridge to work at runtime.

## Next Phase Readiness
- Python bridge ready for integration with Node.js Mac client (Plan 02-03)
- Coprocess bridge script ready to be launched by the Python bridge
- IPC protocol (JSON lines) defined and implemented, ready for Node.js client to connect
- socat installation recommended: `brew install socat`

---
*Phase: 02-terminal-iterm2*
*Completed: 2026-02-04*
