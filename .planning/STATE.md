# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-04)

**Core value:** Full terminal experience remotely - if it works in iTerm2, it should work in the browser
**Current focus:** Phase 2 - Terminal & iTerm2 Integration

## Current Position

Phase: 2 of 3 (Terminal & iTerm2 Integration)
Plan: 3 of 5 in current phase
Status: In progress
Last activity: 2026-02-05 - Completed 02-03-PLAN.md

Progress: [#######---] 67%

## Performance Metrics

**Velocity:**
- Total plans completed: 6
- Average duration: 4 min
- Total execution time: 0.40 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1. Connection & Auth | 3 | 14 min | 5 min |
| 2. Terminal & iTerm2 | 3 | 10 min | 3 min |
| 3. Performance | - | - | - |

**Recent Trend:**
- Last 5 plans: 01-03 (4 min), 02-01 (4 min), 02-02 (3 min), 02-03 (3 min)
- Trend: Stable, fast execution

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

| Decision | Rationale | Plan |
|----------|-----------|------|
| 6-char session codes with nolookalikes alphabet | Balance human-typeable and collision-resistant (~1B combinations) | 01-01 |
| 5-minute code expiry, infinite once paired | Codes expire unused but last forever once connected | 01-01 |
| Zod discriminated unions for protocol | Compile-time + runtime safety for WebSocket messages | 01-01 |
| State machine for connection lifecycle | Validates transitions, prevents invalid state jumps | 01-02 |
| Exponential backoff 1s/2x/30s max with 10% jitter | Balance quick recovery with server protection | 01-02 |
| Svelte 5 runes for reactive state | Modern reactive patterns with $state/$derived/$effect | 01-03 |
| reconnecting-websocket for auto-reconnect | Exponential backoff 1s-30s, max 10 retries | 01-03 |
| Default passthrough routing in relay | All new message types are pure relay - Zod validates, forward raw JSON | 02-01 |
| JSON lines over Unix domain socket for IPC | Simple, debuggable protocol for Python-to-Node.js bridge communication | 02-02 |
| One coprocess socket per session | Avoids multiplexing complexity in coprocess shell script | 02-02 |
| Base64 encoding for terminal data | Raw PTY bytes may contain non-UTF-8, safe for JSON transport | 02-02 |
| fileURLToPath for ESM dirname | tsconfig bundler moduleResolution lacks import.meta.dirname types | 02-03 |

### Pending Todos

None yet.

### Blockers/Concerns

- socat not installed on system; coprocess script falls back to nc. Recommend `brew install socat` for production use.

## Session Continuity

Last session: 2026-02-05
Stopped at: Completed 02-03-PLAN.md
Resume file: None
