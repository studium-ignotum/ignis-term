# Roadmap: iTerm2 Remote

## Overview

This roadmap delivers a complete rebuild of the iTerm2 remote control system. Starting with reliable connection infrastructure and authentication, then building full terminal functionality with iTerm2 tab management, and finally optimizing for production-ready performance. Three phases, 25 requirements, focused on getting a working product.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

- [ ] **Phase 1: Connection & Authentication** - Establish relay infrastructure with session code auth
- [ ] **Phase 2: Terminal & iTerm2 Integration** - Full terminal emulation with tab management
- [ ] **Phase 3: Performance & Reliability** - Production-ready optimization

## Phase Details

### Phase 1: Connection & Authentication
**Goal**: Mac and browser can establish authenticated connections via cloud relay
**Depends on**: Nothing (first phase)
**Requirements**: CONN-01, CONN-02, CONN-03, CONN-04, CONN-05, AUTH-01, AUTH-02, AUTH-03, AUTH-04, AUTH-05
**Success Criteria** (what must be TRUE):
  1. Mac client can start and connect to relay server via WebSocket
  2. Browser can connect to relay and authenticate with session code
  3. Invalid session codes are rejected with clear error message
  4. Connection status is visible in browser UI
  5. Connections auto-reconnect after network interruption
**Plans**: TBD

Plans:
- [ ] 01-01: TBD
- [ ] 01-02: TBD

### Phase 2: Terminal & iTerm2 Integration
**Goal**: Full terminal experience with iTerm2 tab management
**Depends on**: Phase 1
**Requirements**: TERM-01, TERM-02, TERM-03, TERM-04, TERM-05, TERM-06, ITERM-01, ITERM-02, ITERM-03, ITERM-04, ITERM-05
**Success Criteria** (what must be TRUE):
  1. User can see terminal output streaming in real-time in browser
  2. User can type in browser and see input appear in terminal
  3. All terminal features work correctly (colors, cursor, special keys, copy/paste)
  4. User can view list of iTerm2 tabs in sidebar and switch between them
  5. Terminal resizes properly with browser window
**Plans**: TBD

Plans:
- [ ] 02-01: TBD
- [ ] 02-02: TBD
- [ ] 02-03: TBD

### Phase 3: Performance & Reliability
**Goal**: Production-ready performance and resource management
**Depends on**: Phase 2
**Requirements**: PERF-01, PERF-02, PERF-03, PERF-04
**Success Criteria** (what must be TRUE):
  1. Typing feels instant (under 100ms perceived latency)
  2. Long sessions do not consume excessive memory (scrollback bounded)
  3. Disconnected clients are cleaned up properly (no resource leaks)
**Plans**: TBD

Plans:
- [ ] 03-01: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Connection & Authentication | 0/? | Not started | - |
| 2. Terminal & iTerm2 Integration | 0/? | Not started | - |
| 3. Performance & Reliability | 0/? | Not started | - |

---
*Created: 2026-02-04*
*Last updated: 2026-02-04*
