---
phase: 05-mac-client
plan: 03
subsystem: ipc
tags: [unix-socket, ipc, session-tracking, tokio]

# Dependency Graph
requires: ["05-01"]  # Foundation (project structure, dependencies)
provides: ["ipc-server", "session-tracking", "shell-registration-contract"]
affects: ["05-04", "06-*"]  # Integration, Shell Integration Phase

# Tech Tracking
tech-stack:
  added: []  # No new dependencies, uses existing tokio
  patterns: ["unix-domain-socket", "async-accept-loop", "channel-based-events"]

# File Tracking
key-files:
  created:
    - mac-client/src/ipc/mod.rs
    - mac-client/src/ipc/session.rs
  modified:
    - mac-client/src/lib.rs

# Decisions
decisions:
  - id: ipc-socket-path
    choice: "/tmp/terminal-remote.sock"
    rationale: "Standard temp directory, easily discoverable by shell integration"
  - id: stale-socket-cleanup
    choice: "Remove existing socket file on startup"
    rationale: "Prevents address-in-use errors after unclean shutdown"
  - id: session-registration-format
    choice: "JSON with name, shell, pid fields"
    rationale: "Provides context for UI display and debugging"

# Metrics
metrics:
  duration: "~3 minutes"
  completed: "2026-02-05"
---

# Phase 05 Plan 03: IPC Module Summary

**One-liner:** Unix socket IPC server with session tracking and shell registration contract for Phase 6 integration.

## What Was Built

### IPC Server (`mac-client/src/ipc/mod.rs`)

- **UnixListener** bound to `/tmp/terminal-remote.sock`
- **Stale socket cleanup** on startup (removes existing file if present)
- **Drop implementation** for clean shutdown (removes socket file)
- **Async accept loop** spawning task per connection
- **IpcEvent enum** for main thread communication:
  - `SessionConnected { session_id, name }`
  - `SessionDisconnected { session_id }`
  - `SessionCountChanged(usize)`
  - `Error(String)`

### Session Types (`mac-client/src/ipc/session.rs`)

- **Session struct** with id, registration, connected_at
  - `name()` helper for display name access
  - `duration_secs()` for connection duration tracking
- **ShellRegistration struct** (Phase 6 contract):
  - `name: String` - Display name (e.g., "zsh - ~/project")
  - `shell: String` - Shell type (zsh, bash)
  - `pid: u32` - Process ID

## Key Patterns Established

1. **Socket lifecycle management:**
   - Startup: Check for stale socket, remove if exists, bind
   - Shutdown: Drop impl removes socket file

2. **Channel-based event notification:**
   - IPC server -> main thread via `Sender<IpcEvent>`
   - Allows UI updates without shared mutable state

3. **Registration protocol:**
   - First line from client: JSON-serialized `ShellRegistration`
   - Server generates UUID session_id
   - Connection held open until client disconnects

## Commits

| Hash | Type | Description |
|------|------|-------------|
| 0f19af3 | feat | Create IPC server with Unix socket listener |
| 0d62df0 | feat | Add Session duration tracking and ShellRegistration contract |

## Deviations from Plan

None - plan executed exactly as written.

## What's Ready for Next Phase

### For Plan 05-04 (Integration)
- IpcServer ready to be started on background thread
- IpcEvent channel ready for connection to UI updates
- Session tracking ready for state management

### For Phase 6 (Shell Integration)
- Socket path: `/tmp/terminal-remote.sock`
- Registration format: `{"name":"...","shell":"...","pid":123}`
- Connection protocol: Send registration JSON on first line, then bidirectional data

## Verification Results

- [x] `cargo check` passes
- [x] IpcServer has `new()` and `run()` methods
- [x] Session tracking with `HashMap<String, Session>`
- [x] Socket cleanup on Drop
- [x] IpcEvent enum defined for main thread communication
- [x] All 12 tests pass

## Success Criteria Met

- [x] Unix socket binds to /tmp/terminal-remote.sock
- [x] Stale socket removed on startup (prevents "address in use" errors)
- [x] Socket file removed on clean shutdown (Drop impl)
- [x] Session struct tracks id, name, connected_at
- [x] IpcEvent channel communicates session changes to main thread
- [x] ShellRegistration contract defined for Phase 6
