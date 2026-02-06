# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-06)

**Core value:** Control any terminal session from anywhere -- works with any terminal app
**Current focus:** Milestone v2.0 - Rust Rewrite

## Current Position

Phase: 8 - Installer & Setup (1/2 plans complete)
Plan: 08-01 complete
Status: In progress
Last activity: 2026-02-06 -- Completed 08-01-PLAN.md (Installer & Uninstaller)

Progress: [########--] 50% (phase 8)

## v2.0 Overview

Four phases delivering a complete Rust rewrite with universal terminal support:

| Phase | Goal | Requirements | Status |
|-------|------|--------------|--------|
| 4 | Relay Server | 5 (RELAY) | COMPLETE (4/4 plans) |
| 5 | Mac Client | 13 (CLIENT) | COMPLETE (6/6 plans) |
| 6 | Shell Integration | 9 (SHELL) | COMPLETE (2/2 plans) |
| 7 | Web UI & Full Pipeline | 9 (WEB) | IN PROGRESS (2/3 plans) |
| 8 | Installer & Setup | - | IN PROGRESS (1/2 plans) |

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
| Binary frame format: 1-byte length prefix | Variable-length session IDs with simple framing | 05-06 |
| Arc<Mutex<HashMap>> for IPC sessions | Shared mutable access from handler and command processor | 05-06 |
| Clone command senders for forwarding | Each forwarder needs own copy for opposite-module routing | 05-06 |
| winit EventLoop for macOS tray icon | macOS requires proper run loop for tray to appear | 05-05 |
| ApplicationHandler trait pattern | EventLoop::with_user_event() + run_app() for macOS apps | 05-05 |
| TrayIconEvent::set_event_handler bridging | Forward tray events to winit via EventLoopProxy | 05-05 |
| LSUIElement=true for menu bar app | Info.plist key hides app from Dock | 05-05 |
| SMAppService for login items | macOS 13+ API for register/unregister at login | 05-05 |
| nc -U for shell socket communication | Portable across all shells, no external dependencies | 06-01 |
| Background cat to hold socket open | Keeps nc connection alive until explicitly killed | 06-01 |
| Session name format: dirname [PID] | Human-readable, unique per shell instance | 06-01 |
| Prompt hooks for reconnection | Lightweight check every prompt vs dedicated thread | 06-01 |
| Install to ~/.terminal-remote/ | User-local directory, standard pattern for shell tools | 06-02 |
| Source line at END of rc file | Avoids conflicts with oh-my-zsh, starship, p10k | 06-02 |
| 1-byte length prefix for binary frames | Simple framing, max 255-byte session IDs | 07-01 |
| snake_case auth message fields | Match Rust serde(rename_all = "snake_case") | 07-01 |
| Keep v1 Join/Joined messages | Backwards compatibility during transition | 07-01 |
| WebSocket endpoint /ws | Matches Rust relay endpoint | 07-02 |
| Sessions from binary frames | Discover sessions when first binary data arrives | 07-02 |
| Auto-switch first session | Automatically select first arriving session | 07-02 |
| 5-second disconnect removal | Show disconnected briefly, then remove from list | 07-02 |
| writeUtf8 for terminal data | Efficient binary writes in xterm.js 5.x | 07-02 |
| Hard fail on missing Homebrew | No partial installs, cloudflared/tmux require brew | 08-01 |
| Shell integration from release archive | Version-matched init scripts bundled with binaries | 08-01 |
| grep -qF for idempotent source lines | Prevents duplicate entries on re-run | 08-01 |
| LaunchAgent plist for login startup | Simpler than SMAppService, easily reversible | 08-01 |
| Default "n" for piped stdin | curl\|sh can't read interactive input | 08-01 |

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

None -- Shell integration installed and verified working with mac-client.

## Session Continuity

Last session: 2026-02-06T11:08:33Z
Stopped at: Completed 08-01-PLAN.md (Installer & Uninstaller)
Resume file: .planning/phases/08-installer-setup/08-02-PLAN.md
