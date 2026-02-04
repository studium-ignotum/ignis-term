---
phase: 02-terminal-iterm2
plan: 04
subsystem: browser-terminal
tags: [xterm-svelte, xterm.js, webgl, terminal, iterm2-theme, svelte-5-runes]
depends_on:
  requires: [02-01]
  provides: [browser-terminal-component, terminal-store, iterm2-theme-converter, terminal-data-routing]
  affects: [02-05]
tech_stack:
  added: ["@battlefieldduck/xterm-svelte@2.2.1", "@xterm/addon-web-fonts@0.1.0", "@xterm/addon-unicode-graphemes@0.4.0"]
  removed: ["@xterm/xterm@5.5.0", "@xterm/addon-fit@0.10.0"]
  patterns: [xterm-svelte-wrapper, manual-websocket-binding, svelte-5-terminal-store]
key_files:
  created: [src/lib/iterm-theme.ts, src/lib/stores/terminal.svelte.ts]
  modified: [package.json, pnpm-lock.yaml, src/lib/components/Terminal.svelte, src/lib/stores/connection.ts, src/routes/+page.svelte]
decisions:
  - id: "xterm-svelte-types"
    decision: "Import FitAddon/WebglAddon types from @battlefieldduck/xterm-svelte re-exports"
    rationale: "pnpm virtual store prevents direct import of nested @xterm/* type declarations; xterm-svelte re-exports all addon types"
  - id: "terminal-component-exports"
    decision: "Terminal.svelte exports write(), getTerminal(), fit() for external control"
    rationale: "Terminal store needs to write data to registered terminals; exported functions enable this without tightly coupling component internals"
metrics:
  duration: "4 min"
  completed: "2026-02-05"
---

# Phase 2 Plan 4: Browser Terminal Component Summary

Rewrote Terminal.svelte using xterm-svelte with WebGL rendering, created terminal state store with Svelte 5 runes, iTerm2-to-xterm theme converter, and wired connection store to route terminal_data/config messages to the correct terminal instance.

## What Was Done

### Task 1: Install xterm-svelte and create Terminal.svelte with addons (f495f10)

Replaced the old @xterm/xterm + @xterm/addon-fit direct dependencies with @battlefieldduck/xterm-svelte v2.2.1 (which bundles @xterm/xterm v6, @xterm/addon-fit, @xterm/addon-webgl, @xterm/addon-image, @xterm/addon-clipboard, @xterm/addon-web-links, @xterm/addon-search, @xterm/addon-serialize, @xterm/addon-unicode11). Added @xterm/addon-web-fonts and @xterm/addon-unicode-graphemes as separate dependencies.

Rewrote `src/lib/components/Terminal.svelte` (209 lines):
- Uses xterm-svelte `Xterm` component with `bind:terminal` and event props
- Loads addons asynchronously in `onLoad` handler with try/catch per addon:
  - WebglAddon (with `onContextLoss` fallback to DOM renderer)
  - FitAddon (for responsive resize)
  - ClipboardAddon (OSC 52 clipboard support)
  - ImageAddon (sixel + iTerm2 inline image protocol)
  - WebLinksAddon (clickable URLs)
  - Unicode11Addon (emoji/CJK rendering)
- ResizeObserver with 100ms debounced `fitAddon.fit()`
- Guards against zero dimensions and minimum terminal size (20x5)
- Exports `write()`, `getTerminal()`, `fit()` for store integration
- Proper cleanup on destroy (observers, timeouts, addon refs)

### Task 2: Create terminal store, iTerm2 theme converter, wire to WebSocket (0fd3dc2)

Created `src/lib/iterm-theme.ts` (112 lines):
- `defaultTerminalOptions` constant: dark theme, Menlo 14pt, block cursor, 10000 scrollback
- `configToXtermOptions(config: ConfigMessage)`: converts Mac client's ConfigMessage to xterm.js ITerminalOptions
- Builds ITheme from config.theme record with fallback to defaults for missing keys
- Maps cursor style strings to xterm.js 'block' | 'underline' | 'bar'

Created `src/lib/stores/terminal.svelte.ts` (96 lines):
- Svelte 5 runes: `activeSessionId` ($state), `terminalOptions` ($state)
- Terminal registry: Map<sessionId, Terminal> for routing data to correct instance
- Methods: `setActiveSession()`, `applyConfig()`, `registerTerminal()`, `unregisterTerminal()`, `writeData()`, `getTerminal()`
- Exports `terminalStore` singleton

Updated `src/lib/stores/connection.ts` (264 lines):
- Imports terminalStore; routes `terminal_data` -> `terminalStore.writeData()`
- Routes `config` -> `terminalStore.applyConfig()`
- Sets active session on `joined` message
- Placeholder cases for tab management messages (tab_list, tab_switch, tab_created, tab_closed)
- New functions: `sendMessage()` (generic), `sendTerminalInput()`, `sendTerminalResize()`
- Exposes `sessionId` on connectionStore
- Clears active session on disconnect

Updated `src/routes/+page.svelte` (180 lines):
- Renders Terminal component when connected AND activeSessionId exists
- Wires `onInput` -> `sendTerminalInput()`, `onTerminalResize` -> `sendTerminalResize()`
- Compact toolbar with status dot, state text, and disconnect button
- Fullscreen terminal area with flex layout

## Decisions Made

| Decision | Rationale | Impact |
|----------|-----------|--------|
| Import addon types from xterm-svelte re-exports | pnpm virtual store prevents direct @xterm/* type imports | Resolved TypeScript errors without adding peer deps |
| Terminal.svelte exports write/getTerminal/fit | Store needs to write data to specific terminal instances | Enables terminal store to route data without coupling |

## Deviations from Plan

None - plan executed exactly as written.

## Verification Results

1. `pnpm check`: PASS (0 errors, 3 warnings in unrelated files)
2. xterm-svelte installed, old @xterm/xterm and @xterm/addon-fit removed: PASS
3. Terminal.svelte uses WebGL, FitAddon, ClipboardAddon, ImageAddon, WebLinksAddon: PASS
4. Data flow relay -> connection.ts -> terminalStore.writeData -> terminal.write(): PASS
5. Input flow terminal.onData -> onInput -> sendTerminalInput: PASS
6. Resize flow ResizeObserver -> FitAddon.fit -> onResize -> sendTerminalResize: PASS
7. Config message -> terminalStore.applyConfig -> configToXtermOptions: PASS
8. Default theme provides reasonable dark fallback before iTerm2 config: PASS

## Next Phase Readiness

Terminal component and data routing are in place for:
- **02-05**: Tab management UI can use terminalStore.activeSessionId and tab message placeholders
- Terminal renders when connected with active session
- All data flows (input, output, resize, config) are wired end-to-end
