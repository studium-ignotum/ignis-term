# Roadmap: Terminal Remote

## Overview

This roadmap covers two milestones. Milestone v1.0 (Phases 1-3) delivered the initial Node.js/SvelteKit implementation with iTerm2-specific integration. Milestone v2.0 (Phases 4-7) is a complete Rust rewrite with universal terminal support via shell integration, enabling compatibility with any terminal emulator.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

### Milestone v1.0 (Node.js/SvelteKit - Complete)

- [x] **Phase 1: Connection & Authentication** - Establish relay infrastructure with session code auth
- [x] **Phase 2: Terminal & iTerm2 Integration** - Full terminal emulation with tab management
- [ ] **Phase 3: Performance & Reliability** - Production-ready optimization (deferred - starting v2.0)

### Milestone v2.0 (Rust Rewrite)

- [x] **Phase 4: Relay Server** - Rust WebSocket server with embedded web UI
- [x] **Phase 5: Mac Client** - Menu bar app coordinating local sessions with relay
- [ ] **Phase 6: Shell Integration** - Universal terminal support via PTY interposition
- [ ] **Phase 7: Web UI & Full Pipeline** - Browser terminal interface with multi-session support

## Phase Details

### Phase 1: Connection & Authentication (v1.0)
**Status:** Complete
**Goal**: Mac and browser can establish authenticated connections via cloud relay
**Depends on**: Nothing (first phase)
**Requirements**: CONN-01, CONN-02, CONN-03, CONN-04, CONN-05, AUTH-01, AUTH-02, AUTH-03, AUTH-04, AUTH-05
**Success Criteria** (what must be TRUE):
  1. Mac client can start and connect to relay server via WebSocket
  2. Browser can connect to relay and authenticate with session code
  3. Invalid session codes are rejected with clear error message
  4. Connection status is visible in browser UI
  5. Connections auto-reconnect after network interruption

Plans:
- [x] 01-01-PLAN.md -- Shared protocol + relay server (Wave 1)
- [x] 01-02-PLAN.md -- Mac client with reconnection (Wave 2)
- [x] 01-03-PLAN.md -- Browser client + connection status UI (Wave 2)

### Phase 2: Terminal & iTerm2 Integration (v1.0)
**Status:** Complete
**Goal**: Full terminal experience with iTerm2 tab management
**Depends on**: Phase 1
**Requirements**: TERM-01, TERM-02, TERM-03, TERM-04, TERM-05, TERM-06, ITERM-01, ITERM-02, ITERM-03, ITERM-04, ITERM-05
**Success Criteria** (what must be TRUE):
  1. User can see terminal output streaming in real-time in browser
  2. User can type in browser and see input appear in terminal
  3. All terminal features work correctly (colors, cursor, special keys, copy/paste)
  4. User can view list of iTerm2 tabs in sidebar and switch between them
  5. Terminal resizes properly with browser window

Plans:
- [x] 02-01-PLAN.md -- Extended protocol + relay routing (Wave 1)
- [x] 02-02-PLAN.md -- iTerm2 Python bridge + coprocess (Wave 1)
- [x] 02-03-PLAN.md -- Mac client terminal integration (Wave 2)
- [x] 02-04-PLAN.md -- Browser terminal with xterm-svelte (Wave 2)
- [x] 02-05-PLAN.md -- Tab management + mobile UI (Wave 3)

### Phase 3: Performance & Reliability (v1.0)
**Status:** Deferred (starting v2.0 Rust rewrite instead)
**Goal**: Production-ready performance and resource management
**Depends on**: Phase 2
**Requirements**: PERF-01, PERF-02, PERF-03, PERF-04
**Success Criteria** (what must be TRUE):
  1. Typing feels instant (under 100ms perceived latency)
  2. Long sessions do not consume excessive memory (scrollback bounded)
  3. Disconnected clients are cleaned up properly (no resource leaks)

Plans: Deferred

---

### Phase 4: Relay Server (v2.0)
**Status:** Complete
**Goal**: Deployable Rust relay server that routes terminal data between clients
**Depends on**: Nothing (first v2.0 phase)
**Requirements**: RELAY-01, RELAY-02, RELAY-03, RELAY-04, RELAY-05
**Plans:** 4 plans in 4 waves
**Success Criteria** (what must be TRUE):
  1. Single binary runs with embedded web UI accessible at configured port
  2. Mac-client can connect via WebSocket and register a session code
  3. Browser can connect via WebSocket and authenticate with session code
  4. Messages from mac-client are routed to authenticated browser and vice versa
  5. Invalid session codes are rejected with clear error

Plans:
- [x] 04-01-PLAN.md -- Project foundation + protocol types (Wave 1)
- [x] 04-02-PLAN.md -- State management + session codes (Wave 2)
- [x] 04-03-PLAN.md -- Static asset embedding (Wave 3)
- [x] 04-04-PLAN.md -- WebSocket handler + message routing (Wave 4)

### Phase 5: Mac Client (v2.0)
**Status:** Complete
**Goal**: Menu bar app coordinates local sessions with cloud relay
**Depends on**: Phase 4
**Requirements**: CLIENT-01, CLIENT-02, CLIENT-03, CLIENT-04, CLIENT-05, CLIENT-06, CLIENT-07, CLIENT-08, CLIENT-09, CLIENT-10, CLIENT-11, CLIENT-12, CLIENT-13
**Plans:** 6 plans in 4 waves
**Success Criteria** (what must be TRUE):
  1. App runs in menu bar only (no Dock icon visible)
  2. Click icon shows dropdown with session code, connection status, quit option
  3. Session code can be copied to clipboard with confirmation
  4. Unix socket accepts connections from shell integration
  5. Terminal data from local sessions is forwarded to relay

Plans:
- [x] 05-01-PLAN.md -- Project foundation + tray skeleton (Wave 1)
- [x] 05-02-PLAN.md -- WebSocket client module (Wave 2)
- [x] 05-03-PLAN.md -- Unix socket + session tracking (Wave 2)
- [x] 05-04-PLAN.md -- Full integration with clipboard confirmation (Wave 3)
- [x] 05-05-PLAN.md -- App bundle + login items + verification (Wave 4)
- [x] 05-06-PLAN.md -- Bidirectional terminal data forwarding (Wave 4)

### Phase 6: Shell Integration (v2.0)
**Goal**: Any shell session can connect to mac-client for remote access
**Depends on**: Phase 5
**Requirements**: SHELL-01, SHELL-02, SHELL-03, SHELL-04, SHELL-05, SHELL-06, SHELL-07, SHELL-08, SHELL-09
**Plans:** 2 plans in 2 waves
**Success Criteria** (what must be TRUE):
  1. Adding `source ~/.terminal-remote/init.zsh` to .zshrc enables integration
  2. Shell sessions appear as named sessions in mac-client
  3. When mac-client is not running, shell starts normally with no errors or delay
  4. Works with custom prompt themes (starship, p10k, oh-my-zsh)
  5. Shell exit cleanly removes session from mac-client

Plans:
- [ ] 06-01-PLAN.md -- Shell integration scripts for zsh, bash, fish (Wave 1)
- [ ] 06-02-PLAN.md -- Installation and verification (Wave 2)

### Phase 7: Web UI & Full Pipeline (v2.0)
**Goal**: Browser users can view and interact with any connected terminal
**Depends on**: Phase 4, Phase 5, Phase 6
**Requirements**: WEB-01, WEB-02, WEB-03, WEB-04, WEB-05, WEB-06, WEB-07, WEB-08, WEB-09
**Success Criteria** (what must be TRUE):
  1. User enters session code and sees list of connected terminals
  2. Selecting a terminal shows real-time output (colors, cursor, escape sequences)
  3. Keyboard input (including Ctrl+C, arrows, Tab) is sent to terminal
  4. Terminal resizes with browser window and propagates to remote shell
  5. User can switch between multiple terminal sessions

Plans:
- [ ] 07-01-PLAN.md -- TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Connection & Authentication | 3/3 | Complete | 2026-02-04 |
| 2. Terminal & iTerm2 Integration | 5/5 | Complete | 2026-02-05 |
| 3. Performance & Reliability | - | Deferred | - |
| 4. Relay Server | 4/4 | Complete | 2026-02-06 |
| 5. Mac Client | 6/6 | Complete | 2026-02-06 |
| 6. Shell Integration | 0/2 | Planned | - |
| 7. Web UI & Full Pipeline | 0/? | Blocked by Phase 6 | - |

---
*Created: 2026-02-04*
*Last updated: 2026-02-06 (Phase 6 planned - Shell Integration)*
