# Project Research Summary

**Project:** Terminal Remote v2.0 - Rust Rewrite
**Domain:** Remote terminal access with universal terminal support
**Researched:** 2026-02-06
**Confidence:** HIGH

## Executive Summary

Terminal Remote v2.0 is a Rust rewrite of an existing Node.js application that provides remote access to local terminal sessions through a browser. The v2.0 architecture pivots from iTerm2-specific API integration to universal terminal support via shell-level integration, enabling compatibility with any terminal emulator (iTerm2, Terminal.app, VS Code, Warp, Zed, etc.). The recommended approach uses three Rust binaries: a relay server with embedded web UI (axum + rust-embed), a macOS menu bar client (tray-icon + muda), and a shell integration helper (PTY interposition with Unix domain sockets).

The stack is modern and well-documented: axum 0.8 (released Jan 2025) provides WebSocket + HTTP with excellent DX, tray-icon/muda (from the Tauri team) deliver native menu bar integration without full framework overhead, and rust-embed enables single-binary distribution with debug-mode hot reload. The architecture uses PTY interposition to capture terminal I/O universally—a proven pattern from kitty and VS Code shell integrations—combined with Unix domain sockets for efficient local IPC between shell sessions and the menu bar client.

Key risks center on macOS-specific concerns and threading complexity. AppKit's main thread requirement demands careful architecture (Cocoa event loop on main thread, Tokio runtime on background threads, channels for communication). PTY operations are blocking and must use spawn_blocking to avoid hanging the async runtime. Distribution requires proper code signing, notarization with hardened runtime, and TCC permission handling (which resets on code signature changes). Shell integration must coexist with oh-my-zsh/Powerlevel10k using add-zsh-hook patterns rather than direct assignment. These pitfalls are well-documented and avoidable with correct patterns from day one.

## Key Findings

### Recommended Stack

Research confirms a modern Rust stack with strong ecosystem support and recent stable releases. The key insight is using libraries extracted from larger frameworks (tray-icon/muda from Tauri, avoiding the full framework overhead) while leveraging the 2025-2026 Rust web ecosystem momentum behind axum.

**Core technologies:**
- **tray-icon 0.21 + muda 0.17**: Menu bar integration — Tauri-team maintained, actively developed (tray-icon 0.21.3 released Jan 2026), provides exactly what's needed without webview overhead
- **axum 0.8**: WebSocket + HTTP server — Tokio-team framework with superior DX vs actix-web, native WebSocket support, released Jan 2025 with improved ergonomics
- **rust-embed 8.11**: Static asset embedding — Single binary distribution, debug mode reads from filesystem (hot reload), release mode embeds, built-in compression
- **tokio 1.43**: Async runtime — Industry standard, required for axum and tray-icon integration
- **Unix domain sockets (std/tokio)**: Shell IPC — Native macOS support, fast bidirectional communication, no external dependencies needed

**Version confidence:** All versions verified via lib.rs/crates.io on 2026-02-06. Active development continues on all core crates.

**Alternative rejected:** Full Tauri framework (overkill, adds 2-3MB webview), actix-web (steeper learning curve with actor model), cacao (beta quality, overcomplicated for tray-only app), interprocess crate (unnecessary abstraction for macOS-only Unix sockets).

### Expected Features

Research identifies clear table stakes vs differentiators for v2.0 release. The core value proposition is **universal terminal support** (any terminal emulator) replacing v1.0's iTerm2-only approach.

**Must have (table stakes):**
- Menu bar icon with visible session code (6-digit code displayed prominently)
- Copy session code action (clipboard with confirmation)
- Shell auto-connect for zsh (source ~/.terminal-remote/init.zsh integration)
- Silent when mac-client not running (zero overhead, no errors)
- Multi-session sidebar in web UI (see all connected terminals)
- Session switching (click to switch between terminals)
- Connection status indicator (user knows relay connection state)
- No prompt interference (works with starship/p10k/custom prompts)
- Works in any terminal app (iTerm2, Terminal.app, VS Code, Warp, Kitty, Alacritty, etc.)

**Should have (first release):**
- Start at login option (SMAppService API for macOS Ventura+)
- Click session in menu to open browser (quick access)
- Working directory tracking (PWD updates reflected in UI)
- Bash support (second most common shell)
- Session naming from context (PWD or TERM_PROGRAM)
- Graceful disconnect (shell exit cleans up session)

**Defer (v2+):**
- QR code for session code (mobile convenience)
- Fish shell support (smaller user base)
- Command timing/events (start/end tracking)
- Grid view for multiple terminals (2x2, 3x3 layout)
- Session search/filter (needed when 10+ sessions)

**Key insight from research:** Shell-level integration (not terminal API integration) is the proven pattern for universal support. Kitty, VS Code, and iTerm2 all use shell hooks (precmd/preexec) for shell integration features.

### Architecture Approach

The architecture uses PTY interposition to achieve universal terminal support. When a shell starts (via .zshrc integration), a helper binary creates a PTY pair and runs the user's shell on the slave side while relaying I/O through the master to both the terminal emulator and the mac-client via Unix domain socket.

**Major components:**

1. **terminal-remote-attach binary** — Shell integration helper that creates PTY, forks user's shell, relays I/O bidirectionally (stdin/stdout to PTY master, PTY master to Unix socket, socket to PTY master). Uses portable-pty or nix crate for PTY operations with spawn_blocking for async compatibility.

2. **mac-client binary** — Menu bar app that listens on Unix domain socket for session connections, maintains WebSocket connection to relay server, routes messages between local sessions and remote browsers. Uses tray-icon + muda for native UI, tokio for async, channels for main thread communication.

3. **relay-server binary** — WebSocket server with embedded web UI (rust-embed) that routes terminal data between mac-clients (authenticated via session codes) and browsers. Uses axum for HTTP + WebSocket, session code generation (nanoid), message routing with HashMap state.

**Data flow:** Shell output → PTY master → terminal-remote-attach → Unix socket → mac-client → WebSocket → relay → WebSocket → browser (xterm.js). Reverse for input. Binary messages for terminal I/O (no parsing overhead), text messages for control (resize, auth, session list).

**Build order recommendation (from research):**
- Phase 1: Relay server foundation (axum + rust-embed, WebSocket endpoints, session codes)
- Phase 2: Mac client core (menu bar + relay connection, session code display)
- Phase 3: Shell integration (terminal-remote-attach binary, Unix socket client, PTY handling)
- Phase 4: End-to-end data flow (routing through full pipeline, session switching)
- Phase 5: Polish and reliability (reconnection, error recovery, performance)

### Critical Pitfalls

Research identified multiple macOS-specific and Rust async pitfalls that WILL break the app if not addressed correctly from the start.

1. **AppKit Main Thread Violations** — AppKit (NSStatusBar, NSMenu) requires main thread execution. Calling from Tokio worker threads causes crashes. **Mitigation:** Use objc2's MainThreadMarker for compile-time safety, run Tokio on background threads, use channels (mpsc) to send commands from async tasks to main thread. Establish correct threading pattern in Phase 1.

2. **PTY Blocking I/O Hangs** — PTY file descriptors are blocking. Reading from PTY in async context without spawn_blocking freezes entire Tokio runtime. **Mitigation:** Use tokio::task::spawn_blocking for ALL PTY read/write operations, or dedicated blocking threads with channels bridging to async. Design I/O pipeline correctly in Phase 2.

3. **macOS Notarization with Hardened Runtime** — macOS Sequoia requires both code signing AND notarization. Signed-but-not-notarized apps get "damaged and can't be opened" with no bypass. **Mitigation:** Obtain Developer ID certificate ($99/year), sign with --options runtime, submit to notarization (xcrun notarytool), staple ticket. Test full workflow BEFORE release. Set up infrastructure in Phase 1, critical for Phase 5.

4. **TCC Permission Loss on App Updates** — macOS TCC permissions (Accessibility, Automation) are tied to code signature hash. Updates with different signatures revoke previously granted permissions. **Mitigation:** Use consistent Developer ID signing from day one, never change bundle identifier, test permission persistence across updates. Critical for Phase 5 distribution.

5. **Shell Integration Conflicts with Oh-My-Zsh/Powerlevel10k** — Shell hook frameworks compete for precmd/preexec. Direct assignment overwrites existing hooks. **Mitigation:** Use add-zsh-hook function (not direct assignment), load integration AFTER oh-my-zsh, wrap hook code in error handling (fail silently), test extensively with popular configs. Address in Phase 2.

6. **WebSocket Reconnection Not Automatic** — Neither tokio-tungstenite nor axum provide reconnection. Network changes, sleep/wake, proxy timeouts drop connections permanently. **Mitigation:** Implement exponential backoff reconnection on client, heartbeat/ping-pong for health detection, buffer output during disconnection for replay. Build into Phase 3 protocol from start.

**Moderate pitfalls:** SIGWINCH resize not propagated (breaks vim/htop after resize), binary size explosion without strip/LTO, UTF-8 encoding mismatches (emoji/international chars), SMAppService required for login items on Ventura+, universal binary creation is manual (lipo required).

## Implications for Roadmap

Based on research findings, suggested phase structure follows dependency order and risk mitigation strategy. Start with relay (lowest risk, well-documented), then mac-client (establish threading patterns early), then shell integration (highest complexity), then end-to-end integration.

### Phase 1: Relay Server Foundation

**Rationale:** Start with the component that has the highest confidence (axum + rust-embed are well-documented, no macOS-specific concerns, straightforward WebSocket routing). This provides a deployable relay for early testing and establishes the protocol contract.

**Delivers:**
- Deployable relay server binary with embedded web UI
- WebSocket endpoints for mac-client and browser connections
- Session code generation and registration
- Basic message routing between clients
- Static asset serving (xterm.js UI)

**Addresses:**
- Core relay architecture from ARCHITECTURE.md
- rust-embed with compression from STACK.md
- Session code authentication from FEATURES.md (table stakes)

**Avoids:**
- Binary size explosion (Pitfall #8): Configure Cargo.toml release profile from start with strip=true, opt-level="z", lto=true
- rust-embed debug behavior (Pitfall #15): Understand debug vs release asset loading

**Research flags:** NO deeper research needed. Axum WebSocket patterns are well-documented with official examples. rust-embed integration is straightforward.

### Phase 2: Mac Client Core

**Rationale:** Establish correct threading patterns early (main thread for Cocoa, background for Tokio) before adding complexity. This phase validates the most critical architectural decision and exposes macOS-specific issues in isolation.

**Delivers:**
- Menu bar app with tray-icon + muda
- Session code display in menu
- WebSocket client connection to relay
- Copy code to clipboard action
- Connection status indicator
- Unix domain socket listener (foundation for Phase 3)

**Uses:**
- tray-icon 0.21 + muda 0.17 from STACK.md
- tokio-tungstenite for WebSocket client
- Channels for main thread communication

**Avoids:**
- Main thread violations (Pitfall #1): Establish mpsc channel pattern, MainThreadMarker for safety, Tokio on background thread only
- WebSocket reconnection (Pitfall #6): Implement exponential backoff from start, heartbeat for health detection
- Code signing infrastructure (Pitfall #3): Set up Developer ID signing, bundle structure, entitlements early

**Research flags:** MEDIUM research needed for tray-icon event loop integration patterns. Check Tauri menubar examples for event handling best practices.

### Phase 3: Shell Integration

**Rationale:** Most complex component with highest risk (PTY handling, shell hook conflicts, encoding issues). Build after relay and mac-client are validated. Unix socket listener from Phase 2 provides the IPC endpoint.

**Delivers:**
- terminal-remote-attach binary with PTY interposition
- Zsh integration script (.zshrc one-liner)
- Session registration protocol via Unix socket
- I/O relay loop (stdin/stdout <-> PTY <-> socket)
- UTF-8 locale configuration
- Graceful failure when mac-client not running

**Implements:**
- PTY interposition pattern from ARCHITECTURE.md
- Shell hook mechanism from FEATURES.md
- Universal terminal support via shell-level integration

**Avoids:**
- PTY blocking I/O hangs (Pitfall #2): Use spawn_blocking for ALL PTY operations, or dedicated blocking threads with channels
- Oh-My-Zsh conflicts (Pitfall #5): Use add-zsh-hook, load after oh-my-zsh, fail silently with error wrapping
- UTF-8 encoding (Pitfall #9): Set LANG=en_US.UTF-8, LC_ALL=en_US.UTF-8, TERM=xterm-256color on PTY spawn
- Hook error cascades (Pitfall #12): Wrap hook code in { ... } || true, never throw unhandled errors
- Non-interactive shells (Pitfall #13): Guard with [[ -o interactive ]] || return

**Research flags:** HIGH research needed. PTY interposition pattern is established (kitty, VS Code) but needs Rust-specific prototyping. Check portable-pty vs nix crate tradeoffs. Test extensively with oh-my-zsh, Powerlevel10k, starship before finalizing.

### Phase 4: End-to-End Data Flow

**Rationale:** Connect all pieces once each component is individually validated. Focus on message routing, session management, and terminal emulation correctness.

**Delivers:**
- Terminal output routing through full pipeline (shell → browser)
- Browser input routing back to shell
- Session switching in web UI
- Multi-session sidebar display
- Terminal resize propagation (TIOCSWINSZ)
- Session metadata (PWD, shell, terminal app)

**Addresses:**
- Real-time terminal I/O from FEATURES.md (validated in v1)
- Multi-session view from FEATURES.md (new for v2)
- Data flow patterns from ARCHITECTURE.md

**Avoids:**
- SIGWINCH resize not propagated (Pitfall #6): Include resize in protocol, call ioctl(TIOCSWINSZ) on PTY master, test with vim/htop
- WebSocket backpressure (Pitfall #14): Implement flow control, bounded channels with drop-oldest, pause PTY reading when buffer full

**Research flags:** LOW research needed. xterm.js integration validated in v1. Focus on testing and correctness.

### Phase 5: Polish and Distribution

**Rationale:** Final phase addresses macOS distribution requirements, reliability features, and performance optimization. Deferred until core functionality works end-to-end.

**Delivers:**
- Code signing with hardened runtime
- Notarization and stapling
- Universal binary (x86_64 + aarch64)
- Start at login with SMAppService
- Reconnection handling and error recovery
- Performance optimization (batching, compression)
- Installation instructions and shell integration installer

**Avoids:**
- Notarization with hardened runtime (Pitfall #3): Sign with --options runtime, proper Entitlements.plist, submit to notarytool, staple ticket
- TCC permission loss (Pitfall #4): Consistent Developer ID from Phase 1, document which permissions needed, test update scenarios
- SMAppService login items (Pitfall #10): Use smappservice-rs crate, test on Ventura+, handle permission denial gracefully
- Universal binary creation (Pitfall #11): Build for both targets, lipo -create in CI, test on Intel and M1

**Research flags:** MEDIUM research needed for notarization workflow and SMAppService integration. Check Tauri distribution docs and smappservice-rs examples.

### Phase Ordering Rationale

- **Dependencies:** Relay has no dependencies. Mac-client depends on relay protocol. Shell integration depends on mac-client Unix socket. End-to-end depends on all three.
- **Risk mitigation:** Start with highest-confidence component (axum), establish critical patterns early (threading in Phase 2), tackle highest complexity in isolation (PTY in Phase 3).
- **Testability:** Each phase delivers testable artifacts. Relay can be tested with manual WebSocket clients. Mac-client can be tested with mock relay. Shell integration can be tested with mock mac-client socket.
- **Avoiding pitfalls:** Threading patterns established in Phase 2 before PTY complexity. Code signing infrastructure set up early (Phase 2) so Phase 5 isn't blocked. PTY blocking patterns designed correctly in Phase 3 before end-to-end integration.

### Research Flags

**Phases needing deeper research during planning:**

- **Phase 3 (Shell Integration):** PTY interposition in Rust needs prototyping. portable-pty vs nix crate decision requires testing. Oh-My-Zsh/Powerlevel10k compatibility needs validation with actual configs. Consider `/gsd:research-phase` for PTY handling patterns.

- **Phase 5 (Distribution):** Notarization workflow specifics (Entitlements.plist content, notarytool flags) need verification against Apple docs. SMAppService API integration needs code example research. Universal binary CI scripting needs GitHub Actions/cargo-make pattern research.

**Phases with standard patterns (skip research-phase):**

- **Phase 1 (Relay Server):** Axum WebSocket patterns are well-documented with official examples. rust-embed integration is straightforward. No deeper research needed.

- **Phase 2 (Mac Client):** tray-icon + muda patterns available in Tauri menubar examples. WebSocket client with tokio-tungstenite is standard. Threading patterns for Cocoa + Tokio are documented in objc2 examples.

- **Phase 4 (End-to-End):** Extends validated v1 patterns. xterm.js integration known to work. Focus on testing, not research.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All versions verified via lib.rs/crates.io 2026-02-06. Active development on all core crates. Axum 0.8, tray-icon 0.21.3, rust-embed 8.11 all released recently. |
| Features | HIGH | Table stakes validated in v1. Universal terminal support pattern proven by kitty/VSCode/iTerm2 shell integrations. Clear differentiation between must-have and defer. |
| Architecture | MEDIUM-HIGH | PTY interposition pattern is established in other projects. Rust ecosystem verified (portable-pty, tokio Unix sockets). Specific implementation needs prototyping but approach is sound. |
| Pitfalls | HIGH | macOS-specific pitfalls verified with multiple sources (Apple docs, Tauri issues, Rust forum discussions). Clear prevention strategies documented for each. |

**Overall confidence:** HIGH

Research is comprehensive with recent sources (2025-2026), official documentation verification (lib.rs, crates.io, Apple Developer docs), and cross-validation from multiple community sources (Tauri issues, Rust forums, VS Code/kitty integration patterns).

### Gaps to Address

**PTY interposition prototype:** While the pattern is proven in other projects (kitty, VS Code), Rust-specific implementation with portable-pty or nix crate needs prototyping to validate:
- Correct use of spawn_blocking vs dedicated threads
- Error handling for PTY operations
- Raw mode terminal setup
- Signal handling (SIGWINCH, SIGCHLD)

**Recommendation:** Build minimal PTY prototype in Phase 3 planning before committing to full implementation. Test with vim, htop, and nested shells to validate approach.

**Shell hook load ordering:** Oh-My-Zsh and Powerlevel10k have complex initialization sequences. While add-zsh-hook pattern is documented as correct approach, actual integration needs validation with popular configs.

**Recommendation:** Test with fresh oh-my-zsh + Powerlevel10k installation in Phase 3. Document required .zshrc ordering in installation instructions.

**TCC permission UX:** Research confirms permissions reset on code signature changes, but optimal user experience for re-granting permissions after updates is unclear.

**Recommendation:** During Phase 5, design clear UI flow for detecting missing permissions and guiding users to System Preferences. Consider permission checker on app launch.

## Sources

### Primary (HIGH confidence)

**Stack verification:**
- [tray-icon GitHub](https://github.com/tauri-apps/tray-icon) — Tauri team's tray icon library, 0.21.3 released Jan 3, 2026
- [muda lib.rs](https://lib.rs/crates/muda) — Menu utilities, 0.17.1 released Jul 29, 2025
- [axum 0.8.0 announcement](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0) — Official Tokio blog with version specifics
- [rust-embed lib.rs](https://lib.rs/crates/rust-embed) — 8.11.0 released Jan 14, 2026
- [portable-pty documentation](https://docs.rs/portable-pty) — PTY handling crate
- [tokio UnixStream docs](https://docs.rs/tokio/latest/tokio/net/struct.UnixStream.html) — Unix domain socket API

**Architecture patterns:**
- [Kitty Shell Integration](https://sw.kovidgoyal.net/kitty/shell-integration/) — PTY interposition pattern, OSC sequences
- [VS Code Terminal Shell Integration](https://code.visualstudio.com/docs/terminal/shell-integration) — Hook mechanism, precmd/preexec
- [iTerm2 Shell Integration](https://iterm2.com/shell_integration.html) — Shell hook implementation reference

**macOS development:**
- [Apple Developer - Notarization](https://developer.apple.com/documentation/security/resolving-common-notarization-issues) — Official notarization requirements
- [Apple Developer - Developer ID](https://developer.apple.com/developer-id/) — Code signing requirements
- [objc2 MainThreadMarker docs](https://docs.rs/objc2-foundation/latest/objc2_foundation/struct.MainThreadMarker.html) — Thread safety patterns

### Secondary (MEDIUM confidence)

**Pitfalls and solutions:**
- [Tauri Issue #11085](https://github.com/tauri-apps/tauri/issues/11085) — TCC permission loss on updates, confirmed behavior
- [Rust Forum - PTY hangs](https://users.rust-lang.org/t/rust-pty-output-hangs-when-trying-to-read-command-output-in-terminal-emulator/102873) — PTY blocking I/O issue discussion
- [VSCode Issue #146587](https://github.com/microsoft/vscode/issues/146587) — Oh-My-Zsh shell integration conflicts
- [tokio-tungstenite Issue #101](https://github.com/snapview/tokio-tungstenite/issues/101) — WebSocket reconnection discussion
- [min-sized-rust guide](https://github.com/johnthagen/min-sized-rust) — Binary size optimization techniques

**Community resources:**
- [Tauri macOS Bundle docs](https://v2.tauri.app/distribute/macos-application-bundle/) — Distribution best practices
- [smappservice-rs blog post](https://www.gethopp.app/blog/rust-app-start-on-login) — Login item implementation with SMAppService
- [Axum vs Actix-Web 2025](https://medium.com/@indrajit7448/axum-vs-actix-web-the-2025-rust-web-framework-war-performance-vs-dx-17d0ccadd75e) — Framework comparison

### Tertiary (LOW confidence)

**Comparison and context:**
- [What I Learned Building a Native macOS Menu Bar App](https://dev.to/heocoi/what-i-learned-building-a-native-macos-menu-bar-app-4im6) — General menu bar UX insights
- [ttyd - Share terminal over web](https://tsl0922.github.io/ttyd/) — Comparison project for terminal sharing
- [Mosh - Mobile Shell](https://mosh.org) — Reconnection and roaming patterns

---
*Research completed: 2026-02-06*
*Ready for roadmap: yes*
