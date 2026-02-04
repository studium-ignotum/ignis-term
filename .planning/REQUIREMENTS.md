# Requirements: iTerm2 Remote

**Defined:** 2026-02-04
**Core Value:** Full terminal experience remotely — if it works in iTerm2, it should work in the browser.

## v1 Requirements

Requirements for initial release. Complete rebuild of existing broken implementation.

### Connection

- [ ] **CONN-01**: Mac client connects to cloud relay via WebSocket
- [ ] **CONN-02**: Browser connects to cloud relay via WebSocket
- [ ] **CONN-03**: Relay routes messages between Mac and browser
- [ ] **CONN-04**: Connection auto-reconnects on network interruption
- [ ] **CONN-05**: Connection status visible in browser UI

### Authentication

- [ ] **AUTH-01**: Mac client generates session code on startup
- [ ] **AUTH-02**: Session code displayed to user on Mac
- [ ] **AUTH-03**: Browser prompts for session code to connect
- [ ] **AUTH-04**: Invalid session code rejected with clear error
- [ ] **AUTH-05**: Session codes expire after configurable time

### Terminal

- [ ] **TERM-01**: Terminal output streams to browser in real-time
- [ ] **TERM-02**: User input in browser sends to terminal
- [ ] **TERM-03**: Full terminal emulation (colors, cursor, escape sequences)
- [ ] **TERM-04**: Copy/paste works in browser terminal
- [ ] **TERM-05**: Special keys work (Ctrl+C, arrows, Tab)
- [ ] **TERM-06**: Terminal resizes with browser window

### iTerm2 Integration

- [ ] **ITERM-01**: Mac client reads list of open iTerm2 tabs
- [ ] **ITERM-02**: Tab list displays in browser sidebar
- [ ] **ITERM-03**: User can switch between tabs in browser
- [ ] **ITERM-04**: Active tab indicator shows which tab is selected
- [ ] **ITERM-05**: New tabs appear automatically in browser

### Performance

- [ ] **PERF-01**: Incremental terminal updates (not full rewrite)
- [ ] **PERF-02**: Latency under 100ms perceived for typing
- [ ] **PERF-03**: Disconnected clients cleaned up properly
- [ ] **PERF-04**: Terminal scrollback bounded to prevent memory issues

## v2 Requirements

Deferred to future release.

### Polish

- **POLISH-01**: Clickable URLs in terminal output
- **POLISH-02**: Search in scrollback history
- **POLISH-03**: Custom color themes
- **POLISH-04**: Keyboard shortcut parity with iTerm2

### Reliability

- **REL-01**: Session survives browser refresh
- **REL-02**: Terminal state recovery on reconnect
- **REL-03**: Graceful server shutdown

### Mobile

- **MOBILE-01**: Touch-friendly UI on iPad
- **MOBILE-02**: On-screen keyboard integration

## Out of Scope

| Feature | Reason |
|---------|--------|
| User accounts | Single-user tool, session codes sufficient |
| Multi-user collaboration | Not the use case |
| File transfer | Use terminal commands (scp, rsync) |
| Session recording | Enterprise feature, adds complexity |
| Creating new terminals | View/control existing only |
| Local-only mode | Always uses cloud relay |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CONN-01 | TBD | Pending |
| CONN-02 | TBD | Pending |
| CONN-03 | TBD | Pending |
| CONN-04 | TBD | Pending |
| CONN-05 | TBD | Pending |
| AUTH-01 | TBD | Pending |
| AUTH-02 | TBD | Pending |
| AUTH-03 | TBD | Pending |
| AUTH-04 | TBD | Pending |
| AUTH-05 | TBD | Pending |
| TERM-01 | TBD | Pending |
| TERM-02 | TBD | Pending |
| TERM-03 | TBD | Pending |
| TERM-04 | TBD | Pending |
| TERM-05 | TBD | Pending |
| TERM-06 | TBD | Pending |
| ITERM-01 | TBD | Pending |
| ITERM-02 | TBD | Pending |
| ITERM-03 | TBD | Pending |
| ITERM-04 | TBD | Pending |
| ITERM-05 | TBD | Pending |
| PERF-01 | TBD | Pending |
| PERF-02 | TBD | Pending |
| PERF-03 | TBD | Pending |
| PERF-04 | TBD | Pending |

**Coverage:**
- v1 requirements: 25 total
- Mapped to phases: 0 (pending roadmap)
- Unmapped: 25

---
*Requirements defined: 2026-02-04*
*Last updated: 2026-02-04 after initial definition*
