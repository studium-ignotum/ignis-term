# Phase 2: Terminal & iTerm2 Integration - Context

**Gathered:** 2026-02-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Full terminal experience with iTerm2 tab management in the browser. Users can see terminal output streaming in real-time, type and see input, use all terminal features (colors, cursor, special keys, copy/paste), view and manage iTerm2 tabs, and resize the terminal with the browser window.

**Critical requirement:** Must render Claude Code correctly - this is the fidelity benchmark.

</domain>

<decisions>
## Implementation Decisions

### Terminal display
- Use **xterm.js** for browser terminal emulation
- **Match iTerm2 settings** - pull font, colors, cursor style from user's iTerm2 config
- **Match iTerm2 scrollback** - same buffer size as user's iTerm2 configuration
- **Full fidelity rendering** - support inline images (sixel/iTerm2 protocol), ligatures, emoji

### Input handling
- **Match iTerm2 keyboard behavior** - same handling of Cmd+C/V, Ctrl+C, etc.
- **Mouse like iTerm2** - full mouse support for apps like vim/tmux
- **Mobile support required** - touch equivalents for mouse interactions

### Tab sidebar UI
- **Match iTerm2 tab layout** - same position/style as user's iTerm2 tab configuration
- **Match iTerm2 tab info** - same title, icon, status display
- **Full tab management** - can create new tabs and close existing tabs from browser
- **Bidirectional sync** - switching in browser switches iTerm2, switching in iTerm2 updates browser

### Resize behavior
- **Match iTerm2 resize** - same resize behavior as iTerm2

### Claude's Discretion
- Special key handling (arrows, F-keys, Home/End) - optimize for Claude Code usage
- Mobile input approach (virtual keyboard, gestures, floating control bar)
- Minimum terminal size constraints
- Mobile orientation handling (landscape vs portrait)
- Font zoom behavior in browser

</decisions>

<specifics>
## Specific Ideas

- "I want it like iTerm2" - the north star is mirroring iTerm2 behavior as closely as possible
- Must support mobile devices - not just desktop browsers
- Claude Code TUI must render correctly - this validates terminal fidelity

</specifics>

<deferred>
## Deferred Ideas

None - discussion stayed within phase scope

</deferred>

---

*Phase: 02-terminal-iterm2*
*Context gathered: 2026-02-04*
