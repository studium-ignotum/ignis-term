---
phase: 04-relay-server
plan: 04
subsystem: api
tags: [axum, websocket, rust, relay, authentication]

# Dependency graph
requires:
  - phase: 04-02
    provides: AppState with session management
  - phase: 04-03
    provides: Embedded web UI serving
provides:
  - WebSocket handler with client type detection
  - Mac-client registration flow with session codes
  - Browser authentication flow
  - Bidirectional message routing between mac-client and browsers
affects: [05-mac-client, 07-web-ui]

# Tech tracking
tech-stack:
  added: [websocat (dev tool)]
  patterns: [channel-based message routing, split WebSocket sink/stream]

key-files:
  created:
    - relay-server-v2/src/handlers/mod.rs
    - relay-server-v2/src/handlers/ws.rs
    - relay-server-v2/test-ws.sh
  modified:
    - relay-server-v2/src/state.rs
    - relay-server-v2/src/main.rs

key-decisions:
  - "Browser tracking via DashMap in Session struct"
  - "Channel-based message routing (mpsc::channel per client)"
  - "First message determines client type (Register vs Auth)"

patterns-established:
  - "Split WebSocket: separate sender/receiver tasks with channels"
  - "Client type detection via first JSON message"
  - "Browser ID via nanoid for multiple browser support"

# Metrics
duration: 3min
completed: 2026-02-05
---

# Phase 4 Plan 04: WebSocket Handler Summary

**Full WebSocket relay with mac-client registration, browser authentication, and bidirectional message routing via axum split streams and mpsc channels**

## Performance

- **Duration:** 3 min (206 seconds)
- **Started:** 2026-02-05T20:32:11Z
- **Completed:** 2026-02-05T20:35:37Z
- **Tasks:** 3
- **Files modified:** 5

## Accomplishments
- WebSocket handler with client type detection from first message
- Mac-client registration: connects, receives unique session code, broadcasts terminal output
- Browser authentication: validates code, receives terminal output, forwards input to mac-client
- Integration tests validating all flows with websocat CLI tool

## Task Commits

Each task was committed atomically:

1. **Task 1: Create WebSocket handler with client differentiation** - `74bf16e` (feat)
2. **Task 2: Implement mac-client and browser connection flows** - `218509f` (feat)
3. **Task 3: Integration test with websocat** - `a7db7a6` (test)

## Files Created/Modified
- `relay-server-v2/src/handlers/mod.rs` - Handler module exports
- `relay-server-v2/src/handlers/ws.rs` - WebSocket handler with mac-client/browser flows
- `relay-server-v2/src/state.rs` - Added browser tracking and routing methods
- `relay-server-v2/src/main.rs` - Updated to use handlers module
- `relay-server-v2/test-ws.sh` - WebSocket integration test script

## Decisions Made
- **Browser tracking in Session:** Added `browsers: DashMap<String, mpsc::Sender<Vec<u8>>>` to Session struct for managing multiple browser connections per mac-client
- **Channel-based routing:** Each client (mac or browser) gets an mpsc channel for receiving messages, enabling async message forwarding
- **First message protocol:** Client type determined by first JSON message (Register = mac-client, Auth = browser)
- **Case-insensitive codes:** Browser session codes uppercased before validation for user convenience

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- **websocat connection timing:** Initial test failed because websocat `-n1` closes immediately after receiving response, causing session cleanup before browser auth test. Resolved by restructuring tests: invalid code test runs first (doesn't need session), then valid code test uses a persistent mac-client connection with sleep.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Phase 4 (Relay Server) is now complete
- All success criteria from ROADMAP.md verified:
  1. Single binary runs with embedded web UI - VERIFIED (Plan 03)
  2. Mac-client connects via WebSocket and registers - VERIFIED (this plan)
  3. Browser connects and authenticates with session code - VERIFIED (this plan)
  4. Messages route between mac-client and browser - VERIFIED (broadcast_to_browsers, send_to_mac_client)
  5. Invalid session codes rejected with clear error - VERIFIED (AuthFailed response)
- Ready to begin Phase 5: Mac Client

---
*Phase: 04-relay-server*
*Completed: 2026-02-05*
