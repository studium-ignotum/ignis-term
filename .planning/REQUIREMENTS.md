# Requirements: Terminal Remote v2.0

**Defined:** 2026-02-06
**Core Value:** Control any terminal session from anywhere -- works with any terminal app.

## v2.0 Requirements

Requirements for v2.0 Rust rewrite with universal terminal support.

### Relay Server (Rust)

- [x] **RELAY-01**: WebSocket server handles connections from mac-client and browsers
- [x] **RELAY-02**: Session code authentication (generate, validate, expire)
- [x] **RELAY-03**: Message routing between mac-client and paired browser
- [x] **RELAY-04**: Static web UI embedded in binary (rust-embed)
- [x] **RELAY-05**: Single binary distribution with no runtime dependencies

### Mac Client (Rust Menu Bar)

- [x] **CLIENT-01**: Rust binary runs as menu bar app (no Dock icon)
- [x] **CLIENT-02**: WebSocket connection to cloud relay with auto-reconnect
- [x] **CLIENT-03**: Unix socket listener for shell integration IPC
- [x] **CLIENT-04**: Session management (track connected shells)
- [x] **CLIENT-05**: Status icon visible in menu bar
- [x] **CLIENT-06**: Click icon opens dropdown menu
- [x] **CLIENT-07**: Session code displayed in menu
- [x] **CLIENT-08**: Copy session code to clipboard action
- [x] **CLIENT-09**: Connection status indicator (relay connected/disconnected)
- [x] **CLIENT-10**: Quit option in menu
- [x] **CLIENT-11**: Start at login option (Login Items)
- [x] **CLIENT-12**: Template image for menu bar icon (auto dark/light)
- [x] **CLIENT-13**: Session count displayed in menu

### Shell Integration

- [x] **SHELL-01**: Auto-connect to mac-client when shell starts
- [x] **SHELL-02**: Zsh integration script works (`source ~/.terminal-remote/init.zsh`)
- [x] **SHELL-03**: Bash integration script works (`source ~/.terminal-remote/init.bash`)
- [x] **SHELL-04**: Silent no-op when mac-client not running (no errors or delays)
- [x] **SHELL-05**: No prompt interference (works with starship, p10k, custom prompts)
- [x] **SHELL-06**: No perceptible command delay (<10ms added latency)
- [x] **SHELL-07**: Works in any terminal app (iTerm2, VS Code, Zed, Terminal.app, etc.)
- [x] **SHELL-08**: Session named from working directory or terminal app
- [x] **SHELL-09**: Graceful disconnect on shell exit (session removed from mac-client)

### Web UI (Embedded)

- [ ] **WEB-01**: Real-time terminal output streaming via WebSocket
- [ ] **WEB-02**: Keyboard input sent to terminal (including Ctrl+C, arrows, Tab)
- [ ] **WEB-03**: Full terminal emulation with xterm.js (colors, cursor, escape sequences)
- [ ] **WEB-04**: Copy/paste support via browser clipboard
- [ ] **WEB-05**: Connection status indicator visible
- [ ] **WEB-06**: Session code entry form to connect
- [ ] **WEB-07**: Terminal resizes with browser window
- [ ] **WEB-08**: Multi-session sidebar showing all connected terminals
- [ ] **WEB-09**: Session switching (click to view different terminal)

## Future Requirements

Deferred to post-v2.0 release.

### Menu Bar Enhancements

- **MENU-F01**: Session list in dropdown menu
- **MENU-F02**: Click session to open in browser
- **MENU-F03**: QR code for session code
- **MENU-F04**: Notification when new session connects
- **MENU-F05**: Auto-regenerate session code periodically

### Shell Integration Enhancements

- **SHELL-F01**: Fish shell support
- **SHELL-F02**: One-liner installation script
- **SHELL-F03**: tmux/screen detection
- **SHELL-F04**: Working directory tracking (live updates)
- **SHELL-F05**: Command start/end events
- **SHELL-F06**: Process name tracking

### Web UI Enhancements

- **WEB-F01**: Session naming/renaming
- **WEB-F02**: Session metadata display (PWD, app, shell)
- **WEB-F03**: Grid view (multiple terminals visible)
- **WEB-F04**: Session search/filter
- **WEB-F05**: Mobile-responsive sidebar

## Out of Scope

| Feature | Reason |
|---------|--------|
| User accounts | Single-user tool, session codes sufficient |
| Multi-user collaboration | Not the use case |
| Session recording | Privacy concerns, storage complexity |
| File transfer UI | Use terminal commands (scp, rsync) |
| Local-only mode | Always uses cloud relay |
| Create new terminals | View/control existing only |
| Terminal splits in browser | Terminal apps do this better |
| SSH tunneling | Different use case |
| Plugin system | Scope creep |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| RELAY-01 | Phase 4 | Complete |
| RELAY-02 | Phase 4 | Complete |
| RELAY-03 | Phase 4 | Complete |
| RELAY-04 | Phase 4 | Complete |
| RELAY-05 | Phase 4 | Complete |
| CLIENT-01 | Phase 5 | Complete |
| CLIENT-02 | Phase 5 | Complete |
| CLIENT-03 | Phase 5 | Complete |
| CLIENT-04 | Phase 5 | Complete |
| CLIENT-05 | Phase 5 | Complete |
| CLIENT-06 | Phase 5 | Complete |
| CLIENT-07 | Phase 5 | Complete |
| CLIENT-08 | Phase 5 | Complete |
| CLIENT-09 | Phase 5 | Complete |
| CLIENT-10 | Phase 5 | Complete |
| CLIENT-11 | Phase 5 | Complete |
| CLIENT-12 | Phase 5 | Complete |
| CLIENT-13 | Phase 5 | Complete |
| SHELL-01 | Phase 6 | Complete |
| SHELL-02 | Phase 6 | Complete |
| SHELL-03 | Phase 6 | Complete |
| SHELL-04 | Phase 6 | Complete |
| SHELL-05 | Phase 6 | Complete |
| SHELL-06 | Phase 6 | Complete |
| SHELL-07 | Phase 6 | Complete |
| SHELL-08 | Phase 6 | Complete |
| SHELL-09 | Phase 6 | Complete |
| WEB-01 | Phase 7 | Pending |
| WEB-02 | Phase 7 | Pending |
| WEB-03 | Phase 7 | Pending |
| WEB-04 | Phase 7 | Pending |
| WEB-05 | Phase 7 | Pending |
| WEB-06 | Phase 7 | Pending |
| WEB-07 | Phase 7 | Pending |
| WEB-08 | Phase 7 | Pending |
| WEB-09 | Phase 7 | Pending |

**Coverage:**
- v2.0 requirements: 36 total
- Mapped to phases: 36
- Unmapped: 0

---
*Requirements defined: 2026-02-06*
*Last updated: 2026-02-06 after Phase 6 completion*
