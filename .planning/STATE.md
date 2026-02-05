# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-06)

**Core value:** Control any terminal session from anywhere — works with any terminal app
**Current focus:** Milestone v2.0 - Rust Rewrite

## Current Position

Phase: Not started (defining requirements)
Plan: —
Status: Defining requirements
Last activity: 2026-02-06 — Milestone v2.0 started

Progress: [----------] 0%

## v1.0 Summary (Node.js/SvelteKit)

Completed phases 1-2 (connection, auth, terminal, iTerm2 integration).
Phase 3 (performance) deferred — starting v2.0 Rust rewrite instead.

**What worked:**
- WebSocket relay architecture
- Session code authentication
- xterm.js terminal emulation
- Real-time bidirectional I/O

**What's changing:**
- Node.js → Rust (performance, single binary)
- iTerm2-only → Universal (shell integration)
- Separate web app → Embedded in relay

## Accumulated Context

### v1.0 Decisions (may inform v2.0)

| Decision | Rationale | Applies to v2.0? |
|----------|-----------|------------------|
| 6-char session codes | Human-typeable, collision-resistant | ✓ Keep |
| xterm.js for terminal | Industry standard | ✓ Keep (bundle as static) |
| Session codes over passwords | Simple, secure enough | ✓ Keep |
| WebSocket relay | NAT traversal, works anywhere | ✓ Keep |

### Pending Todos

None yet.

### Blockers/Concerns

None yet — fresh start with Rust.

## Session Continuity

Last session: 2026-02-06
Stopped at: Defining v2.0 milestone requirements
Resume file: None
