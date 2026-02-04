# Phase 2: Terminal & iTerm2 Integration - Research

**Researched:** 2026-02-04
**Domain:** Browser terminal emulation (xterm.js), iTerm2 automation/integration, PTY management, mobile terminal UX
**Confidence:** HIGH (verified with npm registries, official docs, GitHub repos, iTerm2 Python API docs)

## Summary

This phase delivers a full browser-based terminal experience that mirrors iTerm2, including real-time terminal I/O, tab management, and configuration synchronization. The core challenge has three dimensions: (1) rendering a high-fidelity terminal in the browser using xterm.js, (2) tapping into existing iTerm2 sessions for bidirectional terminal I/O, and (3) synchronizing tab state between iTerm2 and the browser in real-time.

The standard approach uses:
1. **@xterm/xterm v6.0.0** with WebGL renderer for GPU-accelerated terminal display in the browser
2. **@xterm/addon-fit** for responsive resize, **@xterm/addon-image** for sixel/iTerm2 inline images, **@xterm/addon-webgl** for performance, **@xterm/addon-clipboard** for copy/paste
3. **iTerm2 Python API v0.26** for session discovery, tab management, focus monitoring, configuration reading, and coprocess management
4. **iTerm2 coprocesses** for raw byte-for-byte PTY capture from existing iTerm2 sessions -- the only approach that provides full escape sequence fidelity
5. **node-pty v1.1.0** for creating new terminal sessions when tabs are created from the browser
6. **xterm-svelte v2.2.0** as the Svelte 5 wrapper component for xterm.js

**Primary recommendation:** Use iTerm2 coprocesses (started via the Python API's `session.async_run_coprocess()`) to capture raw PTY bytes from existing sessions, and node-pty for new sessions. A Python subprocess managed by the Mac client bridges Node.js to iTerm2's Python API for all tab management, configuration, and monitoring operations.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| @xterm/xterm | 6.0.0 | Browser terminal emulator | Powers VS Code terminal, 2M+ weekly downloads, full VT100/xterm compat |
| @xterm/addon-webgl | 0.19.0 | GPU-accelerated rendering | 3-5x faster than DOM renderer, essential for high-throughput output |
| @xterm/addon-fit | 0.11.0 | Auto-resize terminal to container | Standard approach for responsive terminal sizing |
| @xterm/addon-image | 0.9.0 | Inline images (sixel + iTerm2 IIP) | Only addon that supports both SIXEL and iTerm2 inline image protocol |
| @xterm/addon-clipboard | 0.2.0 | System clipboard access | OSC 52 clipboard support, custom clipboard providers |
| @xterm/addon-web-fonts | latest | Font loading before render | Ensures custom fonts are loaded before canvas rendering |
| @xterm/addon-web-links | 0.12.0 | Clickable URLs in terminal | Standard URL detection and handling |
| @xterm/addon-unicode-graphemes | latest | Enhanced unicode/emoji support | Grapheme clustering for proper emoji/CJK rendering |
| @battlefieldduck/xterm-svelte | 2.2.0 | Svelte 5 wrapper for xterm.js | Compatible with xterm.js v6 + Svelte 5 runes, manages addon lifecycle |
| node-pty | 1.1.0 | PTY process management (Mac client) | Microsoft-maintained, used by VS Code, macOS native PTY support |
| iterm2 (Python) | 0.26 | iTerm2 automation API | Official API for profiles, sessions, tabs, focus monitoring, coprocesses |
| run-applescript | latest | AppleScript execution from Node.js | Minimal, Promise-based, TypeScript support, AbortSignal |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| @xterm/addon-serialize | 0.14.0 | Terminal buffer serialization | Save/restore terminal state on reconnect |
| @xterm/addon-search | 0.16.0 | Search within terminal buffer | Optional: find text in terminal output |
| @xterm/addon-ligatures | built-in v6 | Font ligature rendering | When user's iTerm2 font has ligatures (e.g., Fira Code) |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| xterm-svelte wrapper | Direct xterm.js integration | Wrapper handles lifecycle, addon management, Svelte 5 bindings; worth the dependency |
| iTerm2 coprocesses | Screen streamer API | Screen streamer gives rendered text, not raw escape sequences; loses fidelity |
| iTerm2 coprocesses | Direct TTY device access | Can't read master side of PTY owned by iTerm2; slave access competes for data |
| iTerm2 Python API | AppleScript only | AppleScript lacks event monitoring (no FocusMonitor), no coprocess control |
| node-pty | child_process.spawn | node-pty provides proper PTY with resize support; child_process has no PTY |
| @xterm/addon-webgl | DOM renderer | DOM renderer is 3-5x slower; WebGL required for smooth scrolling/high throughput |

**Installation:**
```bash
# Browser client (SvelteKit)
npm install @battlefieldduck/xterm-svelte @xterm/addon-web-fonts @xterm/addon-unicode-graphemes

# Mac client
npm install node-pty run-applescript
pip3 install iterm2 pyobjc  # For iTerm2 Python API from Node.js subprocess
```

Note: xterm-svelte v2.2.0 bundles @xterm/xterm@^6.0.0 and many core addons (fit, webgl, image, clipboard, web-links, attach, serialize, search, unicode11, progress). Only addons NOT bundled (web-fonts, unicode-graphemes, ligatures) need separate installation.

## Architecture Patterns

### Recommended Project Structure

```
src/
├── browser/
│   ├── components/
│   │   ├── Terminal.svelte          # xterm.js wrapper via xterm-svelte
│   │   ├── TerminalTabs.svelte      # Tab sidebar matching iTerm2 layout
│   │   ├── MobileControlBar.svelte  # Floating special keys for mobile
│   │   └── ConnectionStatus.svelte  # From Phase 1
│   ├── stores/
│   │   ├── terminal.svelte.ts       # Terminal state ($state runes)
│   │   ├── tabs.svelte.ts           # Tab list, active tab, tab metadata
│   │   └── iterm-config.svelte.ts   # iTerm2 settings (fonts, colors, cursor)
│   └── lib/
│       ├── iterm-theme.ts           # Convert iTerm2 colors → xterm.js ITheme
│       └── terminal-resize.ts       # ResizeObserver + debounced fit
├── mac-client/
│   ├── iterm-bridge.ts              # Manages Python subprocess for iTerm2 API
│   ├── iterm-bridge.py              # Python script: monitors sessions, starts coprocesses
│   ├── coprocess-bridge.sh          # Bridge script run as iTerm2 coprocess
│   ├── session-manager.ts           # Maps iTerm2 sessions to WebSocket streams
│   ├── pty-manager.ts               # node-pty for new sessions created from browser
│   └── config-reader.ts             # Reads iTerm2 profiles (font, colors, cursor)
├── shared/
│   ├── protocol.ts                  # Extended message types for terminal data + tab management
│   └── constants.ts                 # Terminal defaults, resize debounce timing
└── relay/
    └── (from Phase 1)
```

### Pattern 1: iTerm2 Coprocess Bridge (Existing Sessions)

**What:** Attach a coprocess to each iTerm2 session to capture raw PTY bytes and inject input.
**When to use:** For every existing iTerm2 session that needs to be mirrored in the browser.
**Why:** The coprocess stdin receives byte-for-byte PTY output (including escape sequences), which is exactly what xterm.js needs. Coprocess stdout is treated as keyboard input by iTerm2.

```python
# iterm-bridge.py - Python subprocess managed by Mac client
import iterm2
import asyncio

SOCKET_PATH = "/tmp/iterm-bridge.sock"

async def main(connection):
    app = await iterm2.async_get_app(connection)

    # Start coprocess for each existing session
    for window in app.terminal_windows:
        for tab in window.tabs:
            for session in tab.sessions:
                await start_coprocess(session)

    # Monitor for new sessions
    async with iterm2.NewSessionMonitor(connection) as mon:
        while True:
            session_id = await mon.async_get()
            session = app.get_session_by_id(session_id)
            if session:
                await start_coprocess(session)

async def start_coprocess(session):
    """Start a bridge coprocess for an iTerm2 session."""
    session_id = session.session_id
    # Coprocess connects to Mac client via Unix domain socket
    cmd = f"/path/to/coprocess-bridge.sh {session_id} {SOCKET_PATH}"
    success = await session.async_run_coprocess(cmd)
    if not success:
        print(f"Coprocess already running for {session_id}")

iterm2.run_until_complete(main)
```

```bash
#!/bin/bash
# coprocess-bridge.sh - Run as iTerm2 coprocess
# stdin = raw PTY output (byte-for-byte)
# stdout = treated as keyboard input by iTerm2
SESSION_ID="$1"
SOCKET_PATH="$2"

# Connect to Mac client's Unix domain socket
# Forward stdin (PTY output) to socket, socket input to stdout
socat - UNIX-CONNECT:${SOCKET_PATH},session=${SESSION_ID}
```

**Confidence: MEDIUM** - The coprocess approach is architecturally sound (verified via iTerm2 docs), but the bridge script IPC details need prototyping. The `async_run_coprocess()` API is confirmed in the iTerm2 Python API.

### Pattern 2: iTerm2 Configuration Reading

**What:** Read font, colors, cursor style, scrollback from iTerm2 profiles and convert to xterm.js options.
**When to use:** On initial connection and when iTerm2 config changes.
**Why:** The user decision requires matching iTerm2 settings exactly.

```python
# Reading iTerm2 profile configuration
import iterm2

async def get_session_config(connection, session):
    profile = await session.async_get_profile()

    return {
        "font": profile.normal_font,               # e.g., "MesloLGS-NF-Regular 13"
        "nonAsciiFont": profile.non_ascii_font,
        "useLigatures": profile.ascii_ligatures,
        "cursorType": str(profile.cursor_type),     # CURSOR_TYPE_BLOCK, etc.
        "cursorBlink": profile.blinking_cursor,
        "scrollbackLines": profile.scrollback_lines,
        "unlimitedScrollback": profile.unlimited_scrollback,
        "foreground": color_to_hex(profile.foreground_color),
        "background": color_to_hex(profile.background_color),
        "cursor": color_to_hex(profile.cursor_color),
        "ansiColors": [
            color_to_hex(getattr(profile, f"ansi_{i}_color"))
            for i in range(16)
        ],
        "selectionColor": color_to_hex(profile.selection_color),
    }

def color_to_hex(color):
    """Convert iTerm2 Color to hex string for xterm.js ITheme."""
    r = int(color.red * 255)
    g = int(color.green * 255)
    b = int(color.blue * 255)
    return f"#{r:02x}{g:02x}{b:02x}"
```

**Confidence: HIGH** - Verified against iTerm2 Python API profile documentation. All property names confirmed.

### Pattern 3: xterm.js Terminal Setup with iTerm2 Theme

**What:** Configure xterm.js Terminal instance to match iTerm2 settings.
**When to use:** When creating a terminal component in the browser.

```typescript
// Source: xterm.js ITerminalOptions + ITheme official docs
import type { ITerminalOptions, ITheme } from '@xterm/xterm';

interface ITerm2Config {
  font: string;           // "MesloLGS-NF-Regular 13"
  cursorType: string;     // "block" | "underline" | "bar"
  cursorBlink: boolean;
  scrollbackLines: number;
  foreground: string;     // "#d2d2d2"
  background: string;     // "#1e1e1e"
  cursor: string;
  ansiColors: string[];   // 16 colors
  selectionColor: string;
}

function iterm2ToXtermOptions(config: ITerm2Config): ITerminalOptions {
  // Parse font name and size from iTerm2 format "FontName Size"
  const fontParts = config.font.split(' ');
  const fontSize = parseInt(fontParts.pop()!, 10);
  const fontFamily = fontParts.join(' ');

  const theme: ITheme = {
    foreground: config.foreground,
    background: config.background,
    cursor: config.cursor,
    selectionBackground: config.selectionColor,
    black: config.ansiColors[0],
    red: config.ansiColors[1],
    green: config.ansiColors[2],
    yellow: config.ansiColors[3],
    blue: config.ansiColors[4],
    magenta: config.ansiColors[5],
    cyan: config.ansiColors[6],
    white: config.ansiColors[7],
    brightBlack: config.ansiColors[8],
    brightRed: config.ansiColors[9],
    brightGreen: config.ansiColors[10],
    brightYellow: config.ansiColors[11],
    brightBlue: config.ansiColors[12],
    brightMagenta: config.ansiColors[13],
    brightCyan: config.ansiColors[14],
    brightWhite: config.ansiColors[15],
  };

  return {
    fontFamily,
    fontSize,
    cursorStyle: mapCursorType(config.cursorType),
    cursorBlink: config.cursorBlink,
    scrollback: config.scrollbackLines,
    theme,
    allowTransparency: false,
    macOptionIsMeta: false,   // Keep Cmd+C/V for copy/paste on Mac
    customGlyphs: true,       // Better box drawing characters
    drawBoldTextInBrightColors: true,
    scrollOnUserInput: true,
  };
}

function mapCursorType(iterm2Type: string): 'block' | 'underline' | 'bar' {
  switch (iterm2Type) {
    case 'CURSOR_TYPE_UNDERLINE': return 'underline';
    case 'CURSOR_TYPE_VERTICAL': return 'bar';
    default: return 'block';
  }
}
```

**Confidence: HIGH** - Verified against both iTerm2 profile API and xterm.js ITerminalOptions/ITheme docs.

### Pattern 4: Tab Management with Bidirectional Sync

**What:** Monitor iTerm2 tab state and sync with browser; handle browser-initiated tab operations.
**When to use:** Continuous operation while connected.
**Why:** User decision requires bidirectional tab sync.

```python
# iTerm2 tab monitoring (in iterm-bridge.py)
import iterm2

async def monitor_tabs(connection, notify_callback):
    """Monitor tab changes and notify Mac client."""
    app = await iterm2.async_get_app(connection)

    # Monitor focus changes (tab switches)
    async with iterm2.FocusMonitor(connection) as monitor:
        while True:
            update = await monitor.async_get_next_update()
            if update.selected_tab_changed:
                tab_id = update.selected_tab_changed.tab_id
                notify_callback("tab_switched", tab_id)

async def monitor_layout(connection, notify_callback):
    """Monitor tab creation/deletion."""
    async with iterm2.LayoutChangeMonitor(connection) as monitor:
        while True:
            await monitor.async_get()
            # Layout changed - re-enumerate tabs
            app = await iterm2.async_get_app(connection)
            tabs = get_tab_list(app)
            notify_callback("tabs_changed", tabs)

def get_tab_list(app):
    """Get current tab list with metadata."""
    tabs = []
    for window in app.terminal_windows:
        for tab in window.tabs:
            session = tab.current_session
            tabs.append({
                "tab_id": tab.tab_id,
                "session_id": session.session_id if session else None,
                "name": tab.name if hasattr(tab, 'name') else None,
            })
    return tabs

# Browser-initiated operations (called from Mac client via IPC)
async def switch_tab(connection, tab_id):
    app = await iterm2.async_get_app(connection)
    for window in app.terminal_windows:
        for tab in window.tabs:
            if tab.tab_id == tab_id:
                await tab.async_select()
                return True
    return False

async def create_tab(connection, window_id=None):
    app = await iterm2.async_get_app(connection)
    window = app.current_terminal_window
    if window:
        tab = await window.async_create_tab()
        return tab.tab_id
    return None

async def close_tab(connection, tab_id):
    app = await iterm2.async_get_app(connection)
    for window in app.terminal_windows:
        for tab in window.tabs:
            if tab.tab_id == tab_id:
                await tab.async_close()
                return True
    return False
```

**Confidence: HIGH** - FocusMonitor, LayoutChangeMonitor, NewSessionMonitor all verified in iTerm2 Python API docs.

### Pattern 5: Responsive Terminal Resize

**What:** Auto-resize terminal when browser window changes, propagate dimensions to PTY backend.
**When to use:** Always - terminal must fill its container and respond to window resize.
**Why:** TERM-06 requires terminal resizes with browser window.

```typescript
// Source: @xterm/addon-fit documentation + best practices
import { FitAddon } from '@xterm/addon-fit';
import type { Terminal } from '@xterm/xterm';

function setupResponsiveTerminal(
  terminal: Terminal,
  container: HTMLElement,
  onResize: (cols: number, rows: number) => void
) {
  const fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);

  // Debounced resize handler (critical: prevents resize storms)
  let resizeTimeout: ReturnType<typeof setTimeout>;
  function debouncedFit() {
    clearTimeout(resizeTimeout);
    resizeTimeout = setTimeout(() => {
      fitAddon.fit();
    }, 100); // 100ms debounce
  }

  // Use ResizeObserver for container size changes
  const resizeObserver = new ResizeObserver(() => {
    debouncedFit();
  });
  resizeObserver.observe(container);

  // Propagate resize to backend PTY
  terminal.onResize(({ cols, rows }) => {
    onResize(cols, rows);  // Send to Mac client via WebSocket
  });

  // Initial fit
  fitAddon.fit();

  // Cleanup
  return () => {
    clearTimeout(resizeTimeout);
    resizeObserver.disconnect();
  };
}
```

**Confidence: HIGH** - Standard pattern verified across xterm.js documentation, VS Code implementation, and community best practices.

### Pattern 6: WebSocket Terminal Data Flow (Custom, Not addon-attach)

**What:** Manual terminal data binding over WebSocket instead of using the attach addon.
**When to use:** When the WebSocket carries multiplexed data (terminal I/O + tab management + config).
**Why:** The attach addon expects a dedicated WebSocket per terminal. Our relay multiplexes multiple sessions over one connection.

```typescript
// Browser-side: manual terminal ↔ WebSocket binding
import type { Terminal } from '@xterm/xterm';

interface TerminalDataMessage {
  type: 'terminal_data';
  sessionId: string;
  payload: string;  // Raw terminal data (UTF-8)
}

interface TerminalInputMessage {
  type: 'terminal_input';
  sessionId: string;
  payload: string;  // User keystroke data
}

interface TerminalResizeMessage {
  type: 'terminal_resize';
  sessionId: string;
  cols: number;
  rows: number;
}

function bindTerminalToSession(
  terminal: Terminal,
  sessionId: string,
  ws: WebSocket
) {
  // Server → Browser: write terminal output
  function handleMessage(event: MessageEvent) {
    const msg = JSON.parse(event.data);
    if (msg.type === 'terminal_data' && msg.sessionId === sessionId) {
      terminal.write(msg.payload);
    }
  }
  ws.addEventListener('message', handleMessage);

  // Browser → Server: send user input
  const dataDisposable = terminal.onData((data) => {
    const msg: TerminalInputMessage = {
      type: 'terminal_input',
      sessionId,
      payload: data,
    };
    ws.send(JSON.stringify(msg));
  });

  // Browser → Server: send binary input (legacy mouse reports)
  const binaryDisposable = terminal.onBinary((data) => {
    const msg: TerminalInputMessage = {
      type: 'terminal_input',
      sessionId,
      payload: data,
    };
    ws.send(JSON.stringify(msg));
  });

  // Cleanup
  return () => {
    ws.removeEventListener('message', handleMessage);
    dataDisposable.dispose();
    binaryDisposable.dispose();
  };
}
```

**Confidence: HIGH** - This is the standard pattern for multiplexed WebSocket terminals. Verified against xterm.js encoding guide and community examples.

### Anti-Patterns to Avoid

- **Using addon-attach with multiplexed WebSocket:** The attach addon assumes one WebSocket = one terminal. Our architecture multiplexes sessions. Use manual data binding instead.
- **Polling iTerm2 for tab state:** Use FocusMonitor/LayoutChangeMonitor for event-driven updates instead of polling AppleScript in a loop.
- **Creating terminal before container is sized:** Call `terminal.open()` only after the container element has explicit dimensions, then call `fitAddon.fit()`.
- **Not debouncing resize:** Rapid resize events cause erratic terminal behavior and PTY race conditions. Always debounce `fitAddon.fit()` calls.
- **Forgetting terminal.dispose():** xterm.js terminals leak memory if not disposed. Always call `terminal.dispose()` when switching tabs or unmounting components.
- **Mixing text and binary WebSocket frames:** Pick one encoding and stay consistent. Recommend text (UTF-8) for all terminal data.
- **Starting coprocess before container connects:** Coprocess has limited buffering. Start it only when a browser client is connected and ready to receive data.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Terminal emulation | Custom VT100 parser | @xterm/xterm | Thousands of escape sequences, decades of edge cases |
| GPU-accelerated rendering | Custom WebGL terminal renderer | @xterm/addon-webgl | Complex glyph atlas, texture management, context loss handling |
| Terminal resize | Manual col/row calculation | @xterm/addon-fit | Character cell measurement, font metrics, sub-pixel rounding |
| Inline image rendering | Custom image protocol parser | @xterm/addon-image | SIXEL palette handling, iTerm2 IIP base64 decoding, storage management |
| Font loading synchronization | Font load event listeners | @xterm/addon-web-fonts | Race condition between font load and canvas render is subtle |
| iTerm2 tab monitoring | AppleScript polling loop | iTerm2 Python API FocusMonitor | Event-driven, no polling overhead, official async API |
| iTerm2 color conversion | Manual plist parsing | iTerm2 Python API Profile | API returns structured Color objects, handles all color spaces |
| PTY management | child_process.spawn | node-pty | Proper PTY pair creation, resize signal support, flow control |
| Clipboard access | document.execCommand | @xterm/addon-clipboard | OSC 52 protocol support, custom providers, cross-browser |

**Key insight:** Terminal emulation is one of the most edge-case-heavy domains in computing. xterm.js has handled thousands of escape sequence combinations, browser rendering quirks, and input encoding issues over years of development. Any hand-rolled solution will fail on the first complex TUI application (like Claude Code).

## Common Pitfalls

### Pitfall 1: WebGL Context Loss

**What goes wrong:** Terminal goes blank or renders garbage after system sleep, GPU memory pressure, or tab backgrounding.
**Why it happens:** Browsers can drop WebGL contexts at any time (OOM, suspend, Chromium/Nvidia bugs).
**How to avoid:**
1. Listen for `webglcontextlost` event on the canvas
2. Attempt to restore via `clearTextureAtlas()` on the WebGL addon
3. Fall back to DOM renderer if WebGL cannot be restored
4. Re-render terminal content from buffer after context restoration
**Warning signs:** Blank terminal after laptop sleep; garbled characters that fix on scroll.

### Pitfall 2: Font Loading Race Condition

**What goes wrong:** Terminal renders with wrong font (usually a fallback monospace), then snaps to correct font later, causing layout shift and misaligned characters.
**Why it happens:** xterm.js renders to canvas immediately. If the web font hasn't loaded, canvas measures with fallback font metrics. Later font load doesn't trigger re-measure.
**How to avoid:**
1. Use `@xterm/addon-web-fonts` to delay terminal open until fonts are loaded
2. Pre-load the iTerm2 user's font as a web font (serve it from the Mac client)
3. Provide fallback font stack that includes common system monospace fonts
**Warning signs:** Characters overlapping or misaligned on first render; fixes on terminal resize.

### Pitfall 3: FitAddon Resize Collapse (Firefox)

**What goes wrong:** Terminal width collapses to 1 column during rapid resize.
**Why it happens:** Known bug in FitAddon on Firefox/Windows. Container reports 0 width during resize animation.
**How to avoid:**
1. Debounce `fit()` calls with 100ms delay
2. Guard against zero dimensions: `if (container.clientWidth > 0 && container.clientHeight > 0)`
3. Set minimum terminal size constraints (e.g., 20 cols x 5 rows)
**Warning signs:** Terminal shrinks to single column then recovers; flicker during resize.

### Pitfall 4: Coprocess Buffer Blocking

**What goes wrong:** iTerm2 terminal freezes because the coprocess blocked on a full buffer.
**Why it happens:** iTerm2 coprocess has limited buffering. If the Mac client or bridge script doesn't read fast enough, the coprocess blocks, which blocks the PTY, which freezes the terminal.
**How to avoid:**
1. The bridge script must drain stdin continuously and never block
2. Use non-blocking I/O on the Unix domain socket to the Mac client
3. If the browser is disconnected, stop the coprocess gracefully rather than letting it buffer indefinitely
4. Implement flow control: signal the coprocess to pause if downstream is slow
**Warning signs:** iTerm2 terminal hangs; typing produces no output; resolves when browser reconnects.

### Pitfall 5: Terminal Resize Race Condition with PTY

**What goes wrong:** After resize, terminal shows corrupted output - wrong line wrapping, partial escape sequences.
**Why it happens:** Resize triggers SIGWINCH in the PTY. Applications redraw, but xterm.js receives the redraw data interleaved with the resize. Cannot distinguish "old size data" from "new size data."
**How to avoid:**
1. Debounce resize events (100ms)
2. Send resize to PTY backend BEFORE updating xterm.js dimensions (so PTY redraws match new size)
3. Accept minor visual glitches during resize as inherent to terminal emulation (even iTerm2 has them)
**Warning signs:** Garbled lines during resize that fix with `clear` or Ctrl+L.

### Pitfall 6: Copy/Paste Conflicts with Terminal Signals

**What goes wrong:** User presses Ctrl+C to copy text but sends SIGINT to the running process instead.
**Why it happens:** In terminals, Ctrl+C is the interrupt signal, not copy. Users expect browser copy behavior.
**How to avoid:**
1. On macOS: Cmd+C/V works naturally for copy/paste (xterm.js handles this)
2. Set `macOptionIsMeta: false` (default) to preserve Cmd key for browser shortcuts
3. For Linux/Windows browser users: use Ctrl+Shift+C/V (terminal convention)
4. When text is selected, intercept Ctrl+C and perform copy instead of sending SIGINT
5. Use `@xterm/addon-clipboard` for programmatic clipboard access
**Warning signs:** Users accidentally killing processes when trying to copy.

### Pitfall 7: Mobile Virtual Keyboard Overlap

**What goes wrong:** On-screen keyboard covers the terminal, making it impossible to see what you're typing.
**Why it happens:** Mobile browsers push content up or resize the viewport when the keyboard appears. Terminal resize triggers, but visible area is wrong.
**How to avoid:**
1. Use VirtualKeyboard API where available to control keyboard behavior
2. Use `env(keyboard-inset-height)` CSS to account for keyboard
3. Adjust terminal container height when keyboard appears
4. Place the floating control bar between keyboard and terminal
**Warning signs:** Terminal text hidden behind keyboard; terminal resizes to tiny dimensions on mobile.

### Pitfall 8: Memory Leak from Undisposed Terminals

**What goes wrong:** Browser tab memory grows continuously; eventually crashes or becomes sluggish.
**Why it happens:** Switching tabs creates new Terminal instances but doesn't dispose old ones. Event listeners and DOM references keep old terminals in memory.
**How to avoid:**
1. Call `terminal.dispose()` when switching away from a tab
2. Or: keep one Terminal instance and swap the data source (more complex but uses less memory)
3. In Svelte: use `onDestroy` lifecycle to dispose terminal
4. Track all addon instances and dispose them too
**Warning signs:** Memory usage in browser grows steadily; DevTools shows detached DOM elements.

## Code Examples

### Complete Svelte Terminal Component

```svelte
<!-- Source: xterm-svelte docs + xterm.js ITerminalOptions -->
<script lang="ts">
  import { Xterm, XtermAddon } from '@battlefieldduck/xterm-svelte';
  import type { Terminal, ITerminalOptions } from '@battlefieldduck/xterm-svelte';
  import { onDestroy } from 'svelte';

  // Props
  let { sessionId, config, ws, onResize } = $props<{
    sessionId: string;
    config: ITerminalOptions;
    ws: WebSocket;
    onResize: (cols: number, rows: number) => void;
  }>();

  let terminal = $state<Terminal>();
  let cleanup: (() => void) | null = null;

  async function onLoad() {
    if (!terminal) return;

    // Load WebGL renderer (with DOM fallback)
    try {
      const { WebglAddon } = await XtermAddon.WebglAddon();
      const webgl = new WebglAddon();
      webgl.onContextLoss(() => {
        webgl.dispose();
        // Falls back to DOM renderer automatically
      });
      terminal.loadAddon(webgl);
    } catch {
      console.warn('WebGL not available, using DOM renderer');
    }

    // Load FitAddon
    const { FitAddon } = await XtermAddon.FitAddon();
    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);

    // Load clipboard addon
    const { ClipboardAddon } = await XtermAddon.ClipboardAddon();
    terminal.loadAddon(new ClipboardAddon());

    // Responsive resize
    let resizeTimeout: ReturnType<typeof setTimeout>;
    const container = terminal.element?.parentElement;
    if (container) {
      const observer = new ResizeObserver(() => {
        clearTimeout(resizeTimeout);
        resizeTimeout = setTimeout(() => {
          if (container.clientWidth > 0 && container.clientHeight > 0) {
            fitAddon.fit();
          }
        }, 100);
      });
      observer.observe(container);
    }

    // Propagate resize to backend
    terminal.onResize(({ cols, rows }) => {
      onResize(cols, rows);
    });

    // Initial fit
    fitAddon.fit();
  }

  function onData(data: string) {
    // Send user input to Mac client via relay
    ws.send(JSON.stringify({
      type: 'terminal_input',
      sessionId,
      payload: data,
    }));
  }

  // Receive terminal output from Mac client
  function handleMessage(event: MessageEvent) {
    const msg = JSON.parse(event.data);
    if (msg.type === 'terminal_data' && msg.sessionId === sessionId) {
      terminal?.write(msg.payload);
    }
  }

  $effect(() => {
    ws.addEventListener('message', handleMessage);
    return () => ws.removeEventListener('message', handleMessage);
  });

  onDestroy(() => {
    terminal?.dispose();
    cleanup?.();
  });
</script>

<div class="terminal-container">
  <Xterm bind:terminal options={config} {onLoad} {onData} />
</div>

<style>
  .terminal-container {
    width: 100%;
    height: 100%;
    overflow: hidden;
  }
</style>
```

### iTerm2 Color Profile to xterm.js Theme Mapping

```typescript
// Source: iTerm2 Profile API + xterm.js ITheme
// Complete mapping of iTerm2 color properties to xterm.js theme properties

interface ITerm2Color {
  red: number;    // 0.0 - 1.0
  green: number;  // 0.0 - 1.0
  blue: number;   // 0.0 - 1.0
  alpha?: number; // 0.0 - 1.0
}

interface ITerm2Profile {
  foreground_color: ITerm2Color;
  background_color: ITerm2Color;
  cursor_color: ITerm2Color;
  cursor_text_color: ITerm2Color;
  selection_color: ITerm2Color;
  selected_text_color: ITerm2Color;
  ansi_0_color: ITerm2Color;   // black
  ansi_1_color: ITerm2Color;   // red
  ansi_2_color: ITerm2Color;   // green
  ansi_3_color: ITerm2Color;   // yellow
  ansi_4_color: ITerm2Color;   // blue
  ansi_5_color: ITerm2Color;   // magenta
  ansi_6_color: ITerm2Color;   // cyan
  ansi_7_color: ITerm2Color;   // white
  ansi_8_color: ITerm2Color;   // bright black
  ansi_9_color: ITerm2Color;   // bright red
  ansi_10_color: ITerm2Color;  // bright green
  ansi_11_color: ITerm2Color;  // bright yellow
  ansi_12_color: ITerm2Color;  // bright blue
  ansi_13_color: ITerm2Color;  // bright magenta
  ansi_14_color: ITerm2Color;  // bright cyan
  ansi_15_color: ITerm2Color;  // bright white
}

function colorToHex(c: ITerm2Color): string {
  const r = Math.round(c.red * 255);
  const g = Math.round(c.green * 255);
  const b = Math.round(c.blue * 255);
  return `#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`;
}

function profileToTheme(profile: ITerm2Profile): import('@xterm/xterm').ITheme {
  return {
    foreground: colorToHex(profile.foreground_color),
    background: colorToHex(profile.background_color),
    cursor: colorToHex(profile.cursor_color),
    cursorAccent: colorToHex(profile.cursor_text_color),
    selectionBackground: colorToHex(profile.selection_color),
    selectionForeground: colorToHex(profile.selected_text_color),
    black: colorToHex(profile.ansi_0_color),
    red: colorToHex(profile.ansi_1_color),
    green: colorToHex(profile.ansi_2_color),
    yellow: colorToHex(profile.ansi_3_color),
    blue: colorToHex(profile.ansi_4_color),
    magenta: colorToHex(profile.ansi_5_color),
    cyan: colorToHex(profile.ansi_6_color),
    white: colorToHex(profile.ansi_7_color),
    brightBlack: colorToHex(profile.ansi_8_color),
    brightRed: colorToHex(profile.ansi_9_color),
    brightGreen: colorToHex(profile.ansi_10_color),
    brightYellow: colorToHex(profile.ansi_11_color),
    brightBlue: colorToHex(profile.ansi_12_color),
    brightMagenta: colorToHex(profile.ansi_13_color),
    brightCyan: colorToHex(profile.ansi_14_color),
    brightWhite: colorToHex(profile.ansi_15_color),
  };
}
```

### Extended WebSocket Protocol Messages (Terminal + Tabs)

```typescript
// Source: Zod discriminated unions pattern from Phase 1, extended for Phase 2
import { z } from 'zod';

// Terminal I/O messages
const TerminalDataMessage = z.object({
  type: z.literal('terminal_data'),
  sessionId: z.string(),
  payload: z.string(),  // Raw terminal output (UTF-8)
});

const TerminalInputMessage = z.object({
  type: z.literal('terminal_input'),
  sessionId: z.string(),
  payload: z.string(),  // User keystrokes
});

const TerminalResizeMessage = z.object({
  type: z.literal('terminal_resize'),
  sessionId: z.string(),
  cols: z.number().int().positive(),
  rows: z.number().int().positive(),
});

// Tab management messages
const TabInfo = z.object({
  tabId: z.string(),
  sessionId: z.string(),
  title: z.string(),
  isActive: z.boolean(),
});

const TabListMessage = z.object({
  type: z.literal('tab_list'),
  tabs: z.array(TabInfo),
});

const TabSwitchMessage = z.object({
  type: z.literal('tab_switch'),
  tabId: z.string(),
});

const TabCreateMessage = z.object({
  type: z.literal('tab_create'),
});

const TabCloseMessage = z.object({
  type: z.literal('tab_close'),
  tabId: z.string(),
});

const TabCreatedMessage = z.object({
  type: z.literal('tab_created'),
  tab: TabInfo,
});

const TabClosedMessage = z.object({
  type: z.literal('tab_closed'),
  tabId: z.string(),
});

// iTerm2 configuration message
const ConfigMessage = z.object({
  type: z.literal('config'),
  font: z.string(),
  fontSize: z.number(),
  cursorStyle: z.enum(['block', 'underline', 'bar']),
  cursorBlink: z.boolean(),
  scrollback: z.number(),
  theme: z.record(z.string()),  // ITheme as key-value pairs
});

// Union of all Phase 2 messages
const Phase2Message = z.discriminatedUnion('type', [
  TerminalDataMessage,
  TerminalInputMessage,
  TerminalResizeMessage,
  TabListMessage,
  TabSwitchMessage,
  TabCreateMessage,
  TabCloseMessage,
  TabCreatedMessage,
  TabClosedMessage,
  ConfigMessage,
]);
```

## Claude's Discretion Recommendations

These are areas marked as Claude's discretion in the CONTEXT.md decisions.

### Special Key Handling (arrows, F-keys, Home/End)

**Recommendation:** Use xterm.js defaults with one override: set `macOptionIsMeta: false` (default).

xterm.js already handles standard terminal key mappings correctly:
- Arrow keys send ANSI escape sequences (`\x1b[A`, `\x1b[B`, etc.)
- F-keys send standard sequences (`\x1b[11~` through `\x1b[24~`)
- Home/End send `\x1b[H` and `\x1b[F`
- Ctrl+C sends `\x03` (SIGINT)
- Tab sends `\x09`

For Claude Code specifically, no special handling is needed -- Claude Code is a standard terminal application that uses these standard sequences. The one key decision: keep `macOptionIsMeta: false` so that Cmd+C/V works for copy/paste, and Option key produces special characters rather than being Meta.

**Confidence: HIGH**

### Mobile Input Approach

**Recommendation:** Floating special keys control bar above virtual keyboard + standard mobile keyboard for text.

Implement the "Termux/Blink Shell" pattern:
1. **Floating control bar** with: `Esc`, `Ctrl`, `Alt`, `Tab`, arrow keys (`< > ^ v`), `|`, `~`
2. **Sticky modifier behavior**: Tap Ctrl once, it highlights and applies to next keystroke only
3. **Context-aware visibility**: Show control bar only when virtual keyboard is active; hide when external keyboard connected
4. **Touch gestures**: Swipe up/down for scroll, two-finger tap for paste, long press for text selection
5. **Standard mobile keyboard** for all text input

Layout:
```
+---+---+---+---+---+---+---+---+---+---+
| Esc| ^ |Ctrl|Alt|Tab| | | ~ | < | > |
+---+---+---+---+---+---+---+---+---+---+
[          Standard Virtual Keyboard     ]
```

**Confidence: MEDIUM** - Pattern verified across Termux, Blink Shell, and Termius. Implementation specifics need prototyping.

### Minimum Terminal Size Constraints

**Recommendation:** 20 columns x 5 rows minimum.

Below 20 columns, most terminal applications break (prompts wrap unusably, vim becomes unusable). Below 5 rows, there's not enough visible content. If the browser window is too small:
1. Show a "terminal too small" overlay message with current/minimum dimensions
2. Don't resize the terminal below minimums
3. Let the user scroll within the undersized viewport

Claude Code specifically needs at least ~40 columns and ~10 rows to be usable, but the absolute minimum for any terminal functionality is 20x5.

**Confidence: MEDIUM**

### Mobile Orientation Handling

**Recommendation:** Support both landscape and portrait, with landscape preferred.

- **Portrait**: Terminal fills width, control bar + keyboard take bottom half. Limited rows visible (likely 10-15 lines).
- **Landscape**: Terminal gets more columns and rows. Control bar is more compact. Better for actual terminal work.
- **Rotation**: Re-fit terminal on orientation change (debounced). Don't lock orientation.
- **Suggestion banner**: On first mobile connection, suggest landscape mode for better terminal experience.

**Confidence: LOW** - Mobile terminal UX is inherently challenging; needs user testing.

### Font Zoom Behavior

**Recommendation:** Support pinch-to-zoom that changes terminal font size, NOT page zoom.

1. Intercept pinch zoom gesture on the terminal container
2. Adjust xterm.js `fontSize` option (triggers re-render)
3. Re-fit terminal after font size change (new cols/rows)
4. Propagate new cols/rows to PTY backend
5. Clamp font size to reasonable range (8px - 32px)
6. Show brief overlay indicating current font size during zoom
7. Double-tap to reset to iTerm2's configured font size

Note: iTerm2 has a similar feature (Cmd+Plus/Minus for font zoom). Match that behavior in the browser.

**Confidence: MEDIUM**

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `xterm` npm package | `@xterm/xterm` scoped package | v5.4 (March 2024) | Must use scoped packages; old ones deprecated |
| Canvas renderer | WebGL renderer (canvas removed in v6) | v6.0 (Dec 2024) | Canvas addon no longer exists; use WebGL or DOM |
| xterm-addon-ligatures (separate) | Built-in ligature support in v6 | v6.0 (Dec 2024) | v6 has "detailed ligature and variant support" built in |
| iTerm2 AppleScript API | iTerm2 Python API | iTerm2 3.x+ | Python API provides event monitoring, async operations, coprocess control |
| Polling for tab changes | FocusMonitor / LayoutChangeMonitor | iTerm2 Python API | Event-driven, no polling needed |
| Manual font loading | @xterm/addon-web-fonts | Recent | Official addon for web font timing |

**Deprecated/outdated:**
- **`xterm` and `xterm-*` npm packages:** Use `@xterm/*` scoped packages instead
- **`@xterm/addon-canvas`:** Removed in v6.0.0; use WebGL addon or DOM renderer
- **`windowsMode` terminal option:** Removed in v6.0.0
- **`fastScrollModifier` terminal option:** Removed in v6.0.0
- **iTerm2 AppleScript for monitoring:** Use Python API for event-driven operations

## Open Questions

1. **Coprocess ↔ Mac Client IPC mechanism**
   - What we know: Coprocess needs to communicate with Mac client. Options are Unix domain socket, named pipe, or TCP.
   - What's unclear: Best IPC mechanism for low-latency, high-throughput terminal data. Unix domain socket is likely fastest.
   - Recommendation: Prototype with Unix domain socket. The coprocess bridge script can use `socat` or a small binary to connect stdin/stdout to the socket.

2. **Coprocess for existing sessions at startup**
   - What we know: `async_run_coprocess()` can start coprocesses. `NewSessionMonitor` detects new sessions.
   - What's unclear: When the Mac client starts, existing iTerm2 sessions won't have coprocesses yet. Starting one mid-session means missing previous output (scrollback).
   - Recommendation: On Mac client startup, start coprocesses for all sessions. For the initial scrollback, use `session.async_get_contents()` to fetch visible buffer content and send it to the browser. Accept that pre-connection scrollback may have reduced fidelity.

3. **Font serving from Mac client to browser**
   - What we know: User's iTerm2 font may not be available in the browser. Need to serve it as a web font.
   - What's unclear: Legal implications of serving a locally-installed font over the network. Technical approach (read font file from disk, serve via relay?).
   - Recommendation: First try system font name in CSS (works if browser has same font). If not available, fall back to a similar monospace font. Font file serving can be deferred to a later phase.

4. **iTerm2 Python API requires enabling**
   - What we know: The Python API is disabled by default. User must enable it in Preferences > Magic.
   - What's unclear: UX for guiding users through this setup step. Can we detect if it's enabled?
   - Recommendation: Mac client should detect if API is enabled on first connection attempt. If not, show clear instructions. Consider AppleScript as a fallback for basic operations (tab list, tab switch) when Python API is unavailable.

5. **Screen streamer vs. coprocess fidelity**
   - What we know: Coprocess gives raw PTY bytes (full fidelity). Screen streamer gives rendered content (text + attributes per character).
   - What's unclear: Whether screen streamer is "good enough" for most use cases, making it a viable simpler alternative.
   - Recommendation: Start with coprocess approach for full fidelity. Keep screen streamer as a fallback option if coprocess proves problematic.

6. **Performance: multiple concurrent coprocesses**
   - What we know: One coprocess per session. Typical user might have 5-20 tabs.
   - What's unclear: Performance impact of 20 coprocesses each piping data through Unix sockets to the Mac client, especially during heavy output (e.g., `cat` large file, CI output).
   - Recommendation: Benchmark with 10+ sessions during heavy output. May need to implement flow control or only stream the active tab's data at full speed.

## Sources

### Primary (HIGH confidence)
- [npm: @xterm/xterm v6.0.0](https://www.npmjs.com/package/@xterm/xterm) - Current version, API, addons
- [xterm.js GitHub releases](https://github.com/xtermjs/xterm.js/releases) - v6.0.0 breaking changes
- [xterm.js official docs: ITerminalOptions](https://xtermjs.org/docs/api/terminal/interfaces/iterminaloptions/) - Complete options reference
- [xterm.js official docs: ITheme](https://xtermjs.org/docs/api/terminal/interfaces/itheme/) - Complete theme interface
- [xterm.js encoding guide](https://xtermjs.org/docs/guides/encoding/) - UTF-8 handling, binary data
- [xterm.js addons guide](https://xtermjs.org/docs/guides/using-addons/) - Official addon list
- [xterm-addon-image GitHub](https://github.com/jerch/xterm-addon-image) - SIXEL + iTerm2 IIP support details
- [xterm-svelte GitHub](https://github.com/BattlefieldDuck/xterm-svelte) - Svelte 5 wrapper, @xterm/xterm ^6.0.0 dependency confirmed
- [iTerm2 Python API: Profile](https://iterm2.com/python-api/profile.html) - Font, color, cursor, scrollback properties
- [iTerm2 Python API: Session](https://iterm2.com/python-api/session.html) - get_screen_streamer, async_send_text, async_run_coprocess
- [iTerm2 Python API: Focus](https://iterm2.com/python-api/focus.html) - FocusMonitor, tab switch detection
- [iTerm2 Python API: Lifecycle](https://iterm2.com/python-api/lifecycle.html) - NewSessionMonitor, SessionTerminationMonitor, LayoutChangeMonitor
- [iTerm2 Python API: Preferences](https://iterm2.com/python-api/preferences.html) - Global preferences reading
- [iTerm2 Coprocesses documentation](https://iterm2.com/documentation-coprocesses.html) - Byte-for-byte PTY capture
- [iTerm2 Scripting (AppleScript)](https://iterm2.com/documentation-scripting.html) - Tab/session management via AppleScript
- [node-pty GitHub](https://github.com/microsoft/node-pty) - v1.1.0, PTY API, resize support

### Secondary (MEDIUM confidence)
- [xterm.js GitHub issues](https://github.com/xtermjs/xterm.js/issues) - FitAddon resize bugs, mobile touch support limitations, WebGL context loss
- [xterm.js GitHub issue #5377](https://github.com/xtermjs/xterm.js/issues/5377) - Limited mobile touch support (July 2025 proposal)
- [VS Code terminal image PR #182442](https://github.com/microsoft/vscode/pull/182442) - Real-world xterm-addon-image integration
- [Termux extra keys documentation](https://mobile-coding-hub.github.io/termux/customisation/extra_keys/) - Mobile control bar pattern
- [Blink Shell docs](https://docs.blink.sh/) - iOS terminal smart keys pattern
- [VirtualKeyboard API MDN](https://developer.mozilla.org/en-US/docs/Web/API/VirtualKeyboard_API) - Browser virtual keyboard control

### Tertiary (LOW confidence)
- Community blog posts on xterm.js + WebSocket integration patterns
- WebSearch results for mobile terminal UX patterns (verified against Termux/Blink Shell implementations)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All versions verified via npm registry and GitHub releases
- Architecture patterns: MEDIUM-HIGH - Coprocess bridge pattern verified via iTerm2 API docs, but IPC details need prototyping
- iTerm2 integration: HIGH - Python API documentation is comprehensive and current
- Mobile UX: MEDIUM - Patterns verified across established apps, but xterm.js mobile support is limited
- Pitfalls: HIGH - Multiple sources confirm each pitfall (GitHub issues, community reports, official docs)

**Research date:** 2026-02-04
**Valid until:** 2026-03-04 (30 days - xterm.js v6 is stable; iTerm2 API is mature)
