# Project Research Summary

**Project:** Remote Terminal Control Web App
**Domain:** Web-based remote terminal access (iTerm2 relay)
**Researched:** 2026-02-04
**Confidence:** MEDIUM

## Executive Summary

This is a web-based remote control system for iTerm2 that allows users to access their Mac terminal sessions from any browser. The recommended architecture is a three-tier relay system: Mac client (iTerm2 bridge) connects to a cloud relay server, which browsers connect to using session codes. This pattern is standard for terminal sharing tools and solves NAT traversal without exposing the user's home IP.

The core technology stack is well-established: xterm.js for browser terminal emulation (used by VS Code, Hyper, and most web IDEs), SvelteKit for the frontend framework (Svelte 5's runes are excellent for real-time state management), and the lightweight ws library for WebSocket communication. The Mac side uses node-pty for PTY handling and iTerm2's Python API for tab enumeration and control. This combination provides the fastest path to a working prototype with the least complexity.

The key differentiator is native iTerm2 integration - viewing and switching between existing tabs rather than spawning new shells. The main risks are WebSocket state synchronization across three parties (browser, relay, Mac), terminal resize race conditions, and input handling complexity. These are well-documented problems with established mitigation patterns: end-to-end heartbeats, debounced resize with confirmation, and comprehensive keyboard event handling.

## Key Findings

### Recommended Stack

The stack emphasizes minimal abstraction and battle-tested components. xterm.js is the de facto standard for web terminal emulation with full VT100 compatibility and GPU-accelerated rendering via WebGL. SvelteKit provides excellent reactivity for terminal state with minimal bundle size (critical for responsive feel). The ws library offers raw WebSocket protocol without the overhead of socket.io's abstraction layers, which matters for terminal latency.

**Core technologies:**
- **xterm.js v5.x**: Browser terminal emulation - de facto standard with full escape sequence support
- **SvelteKit v2.x + Svelte 5**: Full-stack framework - minimal overhead, excellent for real-time state
- **ws v8.x**: WebSocket server - lightweight, zero dependencies, binary message support
- **Node.js v22 LTS**: Server runtime - stable, good WebSocket support
- **node-pty v1.x**: PTY handling on Mac - proper terminal sizing and signal handling
- **iTerm2 Python API**: Tab enumeration and control - enables key differentiator

**Why NOT alternatives:**
- socket.io: Unnecessary abstraction layer adds latency
- Next.js: React reconciliation overhead bad for terminal rendering
- hterm: Less community adoption than xterm.js

### Expected Features

Feature priorities are organized around table stakes (users expect these) versus differentiators (what makes this product unique).

**Must have (table stakes):**
- Real-time bidirectional I/O - latency under 100ms perceived
- Full terminal emulation - vim/tmux/ncurses must work correctly
- Copy/paste support - universal text interface expectation
- Special key handling - Ctrl+C, arrows, Tab all working
- Secure connection - WSS not WS, users won't trust plaintext
- Session code auth - simple pairing model like AirDrop
- Connection status indicator - user must know connection state
- Basic reconnection handling - network blips shouldn't kill session

**Should have (competitive differentiators):**
- **iTerm2 tab visibility** - see ALL open tabs, not just one session (KEY DIFFERENTIATOR)
- **Tab switching from browser** - control which iTerm2 tab is active (KEY DIFFERENTIATOR)
- Session persistence across disconnects - refresh browser, session survives
- Clickable URLs - links in terminal output open in browser
- Search in scrollback - find text in terminal history
- Custom themes - match user's terminal aesthetic

**Defer (v2+):**
- Session recording/replay - enterprise feature, adds storage complexity
- File transfer/SFTP - scope creep, scp/rsync work fine in terminal
- Multi-user collaboration - not the use case
- Mobile-optimized UI - should work on mobile but not optimized
- Plugin/extension system - massive complexity for focused tool

**Anti-features (explicitly don't build):**
- User accounts/auth system - single-user simplicity is a feature
- Terminal multiplexing - iTerm2 already does this
- Audit logging - enterprise feature with legal implications

### Architecture Approach

The three-tier relay architecture separates concerns cleanly: Mac client handles iTerm2 integration, relay routes messages without interpreting terminal data, browser renders and captures input.

**Major components:**
1. **Mac Client (iTerm2 Bridge)** - Connects to iTerm2 Python API, enumerates tabs/sessions, forwards input to iTerm2, streams output to relay
2. **Cloud Relay (Session Router)** - Manages WebSocket connections from both sides, generates session codes, routes messages bidirectionally, handles heartbeats
3. **Browser (Terminal UI)** - Renders terminal with xterm.js, captures keyboard input, displays session selector, maintains connection state

**Key data flows:**
- Terminal output: iTerm2 -> Mac Client -> Relay -> Browser -> xterm.js render
- Keyboard input: Browser capture -> Relay -> Mac Client -> iTerm2
- Session discovery: Browser requests -> Relay -> Mac Client (enumerates via iTerm2 API) -> Browser
- Session switch: Browser selection -> Relay -> Mac Client (changes active stream)

**Protocol design:**
- JSON messages over WebSocket with type discrimination
- Binary frames for terminal data (performance)
- Lazy session streaming (only active session sends output)
- End-to-end heartbeat (not just relay ping)

### Critical Pitfalls

The research identified 15 pitfalls across critical, moderate, and minor severity. Top priorities:

1. **WebSocket reconnection without state synchronization** - After connection drop, terminal state becomes corrupted. Mid-escape-sequence drops cause garbled output. Prevention: sequence-numbered messages, replay unacknowledged data, use xterm.js serialize addon.

2. **Terminal resize race conditions** - Browser resizes but PTY on Mac doesn't sync. Vim/tmux render garbage. Prevention: debounce resize events (200-300ms), send resize with confirmation, request full redraw after confirmation.

3. **Authentication token exposure in WebSocket URL** - Auth token in query params appears in logs, browser history, proxies. Prevention: use short-lived connection tickets (30 second validity, single use), never log query parameters.

4. **Memory leaks from terminal buffer growth** - Scrollback buffer grows unbounded, browser crashes after hours. Prevention: set explicit scrollback limit (5000 lines), audit parallel data structures, implement circular buffers.

5. **Relay connection state mismatch (three-party problem)** - Browser thinks it's connected but Mac client disconnected. User types into void. Prevention: end-to-end heartbeat, relay notifies browser when Mac disconnects, display state for both connection legs.

6. **Input handling for special keys and modifiers** - Ctrl+C doesn't send interrupt, arrow keys send garbage, copy/paste conflicts. Prevention: comprehensive test matrix for all key combos, use xterm.js attachCustomKeyEventHandler for conflicts.

Additional concerns: WebGL renderer fallback (VM/mobile compatibility), offline/disconnected state handling, blocking operations on message handler (chunk large writes), character encoding mismatches (UTF-8 everywhere).

## Implications for Roadmap

Based on research, suggested phase structure prioritizes foundational connection infrastructure before iTerm2 integration, then polish:

### Phase 1: Core Infrastructure
**Rationale:** WebSocket relay and basic terminal must be rock-solid before adding iTerm2 complexity. All critical connection pitfalls need addressing here.
**Delivers:** Working browser terminal connected to relay with basic auth
**Addresses:**
- Real-time bidirectional I/O (table stakes)
- Secure connection with WSS
- Session code auth flow
- End-to-end heartbeat (avoids Pitfall #5)
- Reconnection with state sync (avoids Pitfall #1)
- UTF-8 encoding throughout (avoids Pitfall #10)
**Avoids:** Three-party state mismatch, auth token exposure, encoding issues
**Research needs:** Standard patterns, minimal research required

### Phase 2: iTerm2 Integration
**Rationale:** Core differentiator that justifies the product. Depends on Phase 1 stability. iTerm2 Python API needs exploration.
**Delivers:** Mac client that connects iTerm2 to relay, tab visibility and switching
**Uses:** node-pty, iTerm2 Python API
**Implements:** Mac Client component from architecture
**Addresses:**
- iTerm2 tab visibility (key differentiator)
- Tab switching from browser (key differentiator)
- Session enumeration and control
**Avoids:** Terminal resize races (Pitfall #2), input handling issues (Pitfall #6)
**Research needs:** HIGH - iTerm2 Python API capabilities need validation, may have undocumented limitations

### Phase 3: Terminal Polish
**Rationale:** Quality of life improvements that make it production-ready. Can iterate incrementally.
**Delivers:** xterm.js addons, keyboard shortcuts, performance optimization
**Addresses:**
- Clickable URLs (xterm-addon-web-links)
- Search in scrollback (xterm-addon-search)
- WebGL with canvas fallback (avoids Pitfall #7)
- Memory limits and monitoring (avoids Pitfall #4)
- Chunked writes for large output (avoids Pitfall #9)
**Avoids:** WebGL compatibility issues, memory leaks, blocking on large output
**Research needs:** LOW - xterm.js addons are well-documented

### Phase 4: UX & Hardening
**Rationale:** User-facing polish and reliability improvements after core functionality works.
**Delivers:** Connection status UI, error recovery, reconnection UX
**Addresses:**
- Connection status indicator (table stakes)
- Offline/disconnected state handling (Pitfall #8)
- Session persistence across disconnects
- Keyboard shortcut parity with iTerm2
**Avoids:** Poor offline handling, confusing error states
**Research needs:** LOW - standard UX patterns

### Phase Ordering Rationale

- **Foundation first:** Both Mac client and browser depend on relay infrastructure. Getting WebSocket communication, message protocol, and auth working is prerequisite for everything else.
- **Differentiator second:** iTerm2 integration is what makes this product unique. After basic terminal works, this becomes the priority. It's also highest risk (API uncertainty).
- **Polish third:** xterm.js addons and optimizations improve quality but don't block core functionality. Can be added incrementally based on user feedback.
- **Hardening last:** Once core features work, focus shifts to reliability and UX refinement.

This ordering also maps to dependency chains from the architecture research: relay enables both client and browser, both enable integration features, integration enables polish.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 2 (iTerm2 Integration):** Complex API with potential undocumented limitations. Should run `/gsd:research-phase` focused on iTerm2 Python API capabilities, session control methods, and tab enumeration patterns.
- **Phase 4 (Deployment):** Relay deployment options (Fly.io vs Railway vs Cloudflare) need cost/latency comparison if targeting production use.

Phases with standard patterns (skip research-phase):
- **Phase 1 (Core Infrastructure):** WebSocket relay patterns are well-established, xterm.js integration is standard, session code auth is simple.
- **Phase 3 (Terminal Polish):** xterm.js addons are well-documented with clear examples.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | MEDIUM-HIGH | xterm.js and ws are battle-tested (HIGH), versions need verification (MEDIUM) |
| Features | HIGH | Table stakes and anti-features clear from competitive landscape |
| Architecture | MEDIUM | Relay pattern is standard (HIGH), iTerm2 API specifics need verification (MEDIUM) |
| Pitfalls | MEDIUM-HIGH | Terminal emulation issues are well-documented (HIGH), three-party edge cases need testing (MEDIUM) |

**Overall confidence:** MEDIUM

All research was conducted using training data knowledge (May 2025 cutoff) without web verification tools. Technology recommendations are sound and based on established patterns, but version numbers and API specifics should be verified before implementation.

### Gaps to Address

Areas where research was inconclusive or needs validation during implementation:

- **iTerm2 Python API capabilities:** Training data has the general API surface, but exact method signatures, async patterns, and edge cases should be verified against current official documentation. Specific questions: Can we get tab titles? How does output streaming perform with multiple sessions? Are there rate limits or throttling?

- **WebSocket reconnection edge cases:** The general patterns are clear, but the specific implementation for three-party reconnection (especially when one leg drops but the other doesn't) needs careful state machine design and testing.

- **Latency characteristics:** Research suggests the architecture should achieve sub-100ms latency, but actual performance depends on relay location, network conditions, and message batching strategy. Should benchmark early.

- **Version verification required:** All npm package versions should be checked with `npm view [package] version` before implementation. Breaking changes may have occurred after training cutoff.

- **Browser keyboard event edge cases:** Research identified the issue but browser-specific behavior (especially Safari vs Chrome vs Firefox) needs hands-on testing. Create comprehensive test matrix.

- **Deployment requirements:** If targeting production use, need to research specific relay deployment options (Fly.io, Railway, Cloudflare Workers) for cost, latency, and WebSocket support characteristics.

## Sources

### Primary (HIGH confidence)
- xterm.js architecture and patterns - widely used in VS Code, Hyper, Theia
- WebSocket relay patterns - standard real-time architecture practices
- Terminal emulation fundamentals - stable domain with decades of established patterns

### Secondary (MEDIUM confidence)
- SvelteKit v2 + Svelte 5 capabilities - training data knowledge
- ws library API and performance characteristics - npm ecosystem knowledge
- iTerm2 Python API surface - based on documentation patterns
- node-pty capabilities - training data knowledge

### Tertiary (LOW confidence - needs validation)
- Specific npm package versions - May 2025 training cutoff, should verify current
- iTerm2 API method signatures - general patterns known, specifics should verify
- Browser-specific keyboard event behavior - needs hands-on testing
- Deployment platform WebSocket support - needs current research

**Research method limitation:** Web verification tools (WebSearch, WebFetch) were unavailable during research. All findings are based on training data knowledge. Before implementation, verify:
1. Current npm package versions
2. iTerm2 Python API documentation
3. Browser compatibility matrices
4. Deployment platform capabilities

---
*Research completed: 2026-02-04*
*Ready for roadmap: yes*
