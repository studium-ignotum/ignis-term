# Terminal Remote

## What This Is

A universal remote terminal control system that lets you access any terminal session on your Mac from any browser, anywhere. A lightweight menu bar app auto-connects your shell sessions (via shell integration), streams them through a cloud relay, and a web UI lets you view and control everything from your iPad, laptop, or phone — regardless of which terminal app you're using.

## Core Value

Control any terminal session from anywhere — works with iTerm2, VS Code, Zed, JetBrains, Terminal.app, anything.

## Current Milestone: v2.0 Rust Rewrite

**Goal:** Rewrite in Rust for performance and single-binary distribution, with universal terminal support via shell integration.

**Target features:**
- Mac client as Rust menu bar app (single binary)
- Relay server in Rust with embedded web UI
- Shell integration that works with any terminal app
- Zero runtime dependencies

## Requirements

### Validated

v1.0 implementation (Node.js/SvelteKit) validated:
- ✓ WebSocket relay architecture works
- ✓ Session code authentication flow
- ✓ xterm.js terminal emulation
- ✓ Real-time bidirectional I/O
- ✓ iTerm2 coprocess integration (limited to iTerm2)

### Active

- [ ] Rust mac-client with menu bar UI
- [ ] Rust relay-server with embedded web UI
- [ ] Shell integration for universal terminal support
- [ ] Single binary distribution (no Python/Node.js)
- [ ] Auto-connect any terminal session when mac-client running

### Out of Scope

- User accounts / multi-user — single user, session codes only
- Mobile native apps — web only
- Local-only mode — always uses cloud relay
- Per-terminal adapters — shell integration is universal approach

## Context

**v1.0 implementation:** Working prototype in Node.js/SvelteKit with iTerm2-specific coprocess integration. Validates the architecture but limited to one terminal app and requires Node.js runtime.

**v2.0 approach:** Rust rewrite with shell integration. Instead of per-terminal APIs, inject at shell level — one line in .zshrc auto-connects any session when mac-client is running.

**Architecture:** Mac (menu bar app) → Cloud Relay (Rust, serves web UI) ← Browser. Shell integration script in .zshrc connects sessions to mac-client via Unix socket.

**Terminal emulation:** Keep xterm.js for browser-side rendering. Bundled as static assets in Rust relay binary.

## Constraints

- **Tech stack**: Rust for mac-client and relay-server
- **Terminal lib**: xterm.js bundled as static assets
- **Auth model**: Session codes only, no user accounts
- **Shell**: Zsh (user's shell), integration script in .zshrc
- **Distribution**: Single binary per component, no runtime dependencies

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust rewrite | Performance + single binary distribution | — Pending |
| Shell integration over terminal adapters | Universal approach, works with any terminal app | — Pending |
| Menu bar app | Lightweight, always accessible, shows session code | — Pending |
| Embedded web UI | Single relay binary serves everything | — Pending |
| Keep xterm.js | Proven terminal emulation, bundle as static assets | — Pending |

---
*Last updated: 2026-02-06 after v2.0 milestone start*
