# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-06)

**Core value:** Control any terminal session from anywhere -- works with any terminal app
**Current focus:** Milestone v2.0 - Rust Rewrite

## Current Position

Phase: 4 - Relay Server (1/4 plans complete)
Plan: 01 complete, 02 next
Status: In progress
Last activity: 2026-02-05 -- Completed 04-01-PLAN.md (project initialization)

Progress: [##--------] 25% (phase 4)

## v2.0 Overview

Four phases delivering a complete Rust rewrite with universal terminal support:

| Phase | Goal | Requirements | Status |
|-------|------|--------------|--------|
| 4 | Relay Server | 5 (RELAY) | In progress (1/4 plans) |
| 5 | Mac Client | 13 (CLIENT) | Blocked by Phase 4 |
| 6 | Shell Integration | 9 (SHELL) | Blocked by Phase 5 |
| 7 | Web UI & Full Pipeline | 9 (WEB) | Blocked by Phase 6 |

## v1.0 Summary (Node.js/SvelteKit)

Completed phases 1-2 (connection, auth, terminal, iTerm2 integration).
Phase 3 (performance) deferred -- starting v2.0 Rust rewrite instead.

**What worked:**
- WebSocket relay architecture
- Session code authentication
- xterm.js terminal emulation
- Real-time bidirectional I/O

**What's changing:**
- Node.js -> Rust (performance, single binary)
- iTerm2-only -> Universal (shell integration)
- Separate web app -> Embedded in relay

## Accumulated Context

### v1.0 Decisions (apply to v2.0)

| Decision | Rationale | Applies to v2.0? |
|----------|-----------|------------------|
| 6-char session codes | Human-typeable, collision-resistant | Keep |
| xterm.js for terminal | Industry standard | Keep (bundle as static) |
| Session codes over passwords | Simple, secure enough | Keep |
| WebSocket relay | NAT traversal, works anywhere | Keep |

### v2.0 Decisions

| Decision | Rationale | Phase |
|----------|-----------|-------|
| axum-embed 0.1 | 0.2 not available, 0.1 works | 04-01 |
| Tagged enum protocol | `#[serde(tag = "type", rename_all = "snake_case")]` for clean JSON | 04-01 |
| PORT env with default | `std::env::var("PORT").unwrap_or_else(\|_\| "3000")` | 04-01 |

### v2.0 Stack (from research)

| Component | Technology | Notes |
|-----------|------------|-------|
| Menu bar | tray-icon 0.21 + muda 0.17 | Tauri-team maintained |
| WebSocket server | axum 0.8 | Tokio-team framework |
| Static embedding | rust-embed 8.11 | Single binary distribution |
| Shell IPC | Unix domain sockets | Native tokio support |
| PTY handling | portable-pty or nix | Needs prototyping |

### Critical Pitfalls (from research)

1. **AppKit main thread** -- Tokio must run on background thread, use channels
2. **PTY blocking I/O** -- Use spawn_blocking for all PTY operations
3. **Notarization required** -- Sign + notarize before distribution
4. **Shell hook conflicts** -- Use add-zsh-hook, load after oh-my-zsh

### Pending Todos

None.

### Blockers/Concerns

None -- relay server foundation established.

## Session Continuity

Last session: 2026-02-05T20:21:10Z
Stopped at: Completed 04-01-PLAN.md
Resume file: .planning/phases/04-relay-server/04-02-PLAN.md
