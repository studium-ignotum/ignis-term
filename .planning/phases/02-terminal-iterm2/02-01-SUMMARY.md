---
phase: 02-terminal-iterm2
plan: 01
subsystem: protocol
tags: [zod, websocket, protocol, relay, terminal, tabs]
depends_on:
  requires: [01-01]
  provides: [terminal-protocol, tab-protocol, config-protocol, relay-routing]
  affects: [02-02, 02-03, 02-04, 02-05]
tech_stack:
  added: []
  patterns: [passthrough-routing, discriminated-union-extension]
key_files:
  created: []
  modified: [src/shared/protocol.ts, src/shared/constants.ts, src/relay/server.ts]
decisions:
  - id: "default-passthrough-routing"
    decision: "Use default switch case for passthrough routing instead of explicit cases per message type"
    rationale: "All new message types are pure passthrough - relay validates via Zod then forwards raw JSON. Avoids listing every type explicitly and auto-handles future message types."
  - id: "zod-v4-record-syntax"
    decision: "Use z.record(z.string(), z.string()) for theme config"
    rationale: "Zod v4 requires both key and value schema arguments for z.record()"
metrics:
  duration: "4 min"
  completed: "2026-02-05"
---

# Phase 2 Plan 1: Protocol Extension & Relay Routing Summary

Extended WebSocket protocol with 10 new Zod-validated message types for terminal I/O, tab management, and iTerm2 config sync; updated relay server with passthrough routing for all new types.

## What Was Done

### Task 1: Extend protocol schemas and constants (8530e0e)

Added terminal-specific constants to `src/shared/constants.ts`:
- `TERMINAL_RESIZE_DEBOUNCE_MS` (100ms)
- `TERMINAL_MIN_COLS` (20), `TERMINAL_MIN_ROWS` (5)
- `TERMINAL_DEFAULT_SCROLLBACK` (10000)

Added 10 new Zod schemas to `src/shared/protocol.ts`:

**Terminal I/O (3 types):**
- `TerminalDataMessage` - Mac to Browser: raw terminal output with sessionId
- `TerminalInputMessage` - Browser to Mac: user keystrokes with sessionId
- `TerminalResizeMessage` - Browser to Mac: cols/rows dimensions with sessionId

**Tab Management (7 types):**
- `TabInfo` - Reusable schema: tabId, sessionId, title, isActive
- `TabListMessage` - Mac to Browser: full tab array
- `TabSwitchMessage` - Bidirectional: switch active tab by tabId
- `TabCreateMessage` - Browser to Mac: create new tab
- `TabCloseMessage` - Browser to Mac: close tab by tabId
- `TabCreatedMessage` - Mac to Browser: new tab notification with TabInfo
- `TabClosedMessage` - Mac to Browser: tab closed notification

**Config (1 type):**
- `ConfigMessage` - Mac to Browser: font, fontSize, cursorStyle, cursorBlink, scrollback, theme

Updated both `IncomingMessage` and `OutgoingMessage` discriminated unions to include all 10 new types.

### Task 2: Update relay server routing (a93fe2b)

Modified `src/relay/server.ts` to route new message types between paired connections:

- **Mac handler**: Replaced default error response with passthrough that forwards any Zod-validated message to the paired browser connection
- **Browser handler**: Replaced default error response with passthrough that forwards any Zod-validated message to the paired Mac connection
- Existing explicit cases (`data`, `ping`, `register` for Mac; `join`, `data`, `ping`, `register` for Browser) remain unchanged
- Auth flow (register/registered, join/joined) completely unaffected
- Invalid messages still rejected by Zod parser before reaching switch statement

## Decisions Made

| Decision | Rationale | Impact |
|----------|-----------|--------|
| Default passthrough routing | All new types are pure relay - validate then forward raw JSON. Avoids explicit cases per type. | Future message types auto-route without relay changes |
| Zod v4 record syntax | z.record() requires 2 args in Zod v4 (key + value schema) | Used z.record(z.string(), z.string()) for theme |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed Zod v4 z.record() API**
- **Found during:** Task 1 verification
- **Issue:** Plan specified `z.record(z.string())` but Zod v4.3.6 requires both key and value schemas
- **Fix:** Changed to `z.record(z.string(), z.string())`
- **Files modified:** src/shared/protocol.ts
- **Commit:** 8530e0e

## Verification Results

- TypeScript strict compilation: PASS (zero source errors)
- All 10 new schemas exported: PASS
- IncomingMessage union updated: PASS (14 types total)
- OutgoingMessage union updated: PASS (15 types total)
- Relay server compiles: PASS
- Existing auth flow preserved: PASS (explicit cases unchanged)
- Server line count: 347 (exceeds min_lines: 100)

## Next Phase Readiness

All protocol types and relay routing are in place for:
- **02-02**: Mac client can use TerminalDataMessage, TabListMessage, etc.
- **02-03**: Browser terminal rendering can consume TerminalDataMessage, ConfigMessage
- **02-04**: Tab UI can use TabListMessage, TabSwitchMessage, etc.
- **02-05**: Resize handling uses TerminalResizeMessage with TERMINAL_RESIZE_DEBOUNCE_MS
