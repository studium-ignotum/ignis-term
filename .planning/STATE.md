# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-06)

**Core value:** Control any terminal session from anywhere -- works with any terminal app
**Current focus:** Milestone v2.0 - Rust Rewrite

## Current Position

Phase: 5 - Mac Client (4/6 plans complete)
Plan: 04 complete
Status: In progress
Last activity: 2026-02-05 -- Completed 05-04-PLAN.md (Integration)

Progress: [######----] 67% (phase 5)

## v2.0 Overview

Four phases delivering a complete Rust rewrite with universal terminal support:

| Phase | Goal | Requirements | Status |
|-------|------|--------------|--------|
| 4 | Relay Server | 5 (RELAY) | COMPLETE (4/4 plans) |
| 5 | Mac Client | 13 (CLIENT) | In progress (4/6 plans) |
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
| 31-char code alphabet | Excludes 0/O/1/I/L to prevent transcription errors | 04-02 |
| Arc<Inner> for AppState | Cheap Clone for axum State while sharing data | 04-02 |
| DashMap for sessions | Lock-free concurrent access without mutex management | 04-02 |
| ServeEmbed with explicit index | `Some("index.html")` required for root path serving | 04-03 |
| FallbackBehavior::Ok for SPA | Returns index.html for unknown paths (client routing) | 04-03 |
| Browser tracking in Session | DashMap<browser_id, Sender> for multiple browsers per session | 04-04 |
| Channel-based message routing | mpsc::channel per client for async message forwarding | 04-04 |
| First message determines client type | Register = mac-client, Auth = browser | 04-04 |
| tray-icon + muda for menu bar | Lighter weight than full Tauri for menu-only app | 05-01 |
| Template icon with icon_as_template(true) | macOS auto-inverts for dark mode | 05-01 |
| Polling event loop with 10ms sleep | Non-blocking try_recv() avoids busy-waiting | 05-01 |
| UiCommand/BackgroundEvent enums | Channel types for future thread communication | 05-01 |
| std::sync::mpsc for relay events | AppKit compatibility - main thread runs event loop | 05-02 |
| Max backoff 32 seconds | 5 doublings (2^5) before capping | 05-02 |
| Protocol types duplicated | Type safety without shared crate complexity | 05-02 |
| IPC socket at /tmp/terminal-remote.sock | Standard temp dir, discoverable by shell integration | 05-03 |
| Stale socket cleanup on startup | Prevents address-in-use after unclean shutdown | 05-03 |
| JSON shell registration format | name/shell/pid fields for UI display and debugging | 05-03 |
| spawn_blocking for event forwarding | Bridges async Tokio with sync main thread mpsc | 05-04 |
| AppState holds MenuItem refs | Allows set_text() calls for dynamic menu updates | 05-04 |
| 2-second Copied! feedback | Polling reset in event loop, simple implementation | 05-04 |
| BackgroundCommand::Shutdown | Graceful shutdown via channel signaling | 05-04 |

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

Last session: 2026-02-05T21:36:38Z
Stopped at: Completed 05-04-PLAN.md (Integration)
Resume file: .planning/phases/05-mac-client/05-05-PLAN.md
