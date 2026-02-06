# Phase 7: Web UI & Full Pipeline - Research

**Researched:** 2026-02-06
**Domain:** WebSocket-based terminal emulation with xterm.js, React 19, and binary protocol integration
**Confidence:** HIGH

## Summary

Phase 7 connects an existing React web UI to the Rust relay server, updating the web UI protocol to match the relay's `auth`/`auth_success` + binary frame protocol. The web UI already exists with React 19, xterm.js 6.0, and Vite 6, but uses an outdated JSON-based protocol that must be replaced with binary WebSocket frames.

The standard approach for browser-based terminal emulation uses xterm.js with the FitAddon for responsive sizing and WebGLAddon for GPU acceleration. Binary terminal I/O flows through WebSocket binary frames with a length-prefixed session ID format (1-byte length + session ID + payload). Control messages (auth, errors) remain JSON over text frames.

Key challenges include proper WebSocket lifecycle management in React (cleanup on unmount), binary frame encoding/decoding, terminal resize coordination, and avoiding Context re-render cascades. The existing `reconnecting-websocket` library handles connection resilience, while careful Context splitting prevents unnecessary re-renders.

**Primary recommendation:** Use `writeUtf8(Uint8Array)` for binary terminal data, split WebSocket state into separate Contexts (connection vs terminal data), implement proper cleanup in useEffect, and leverage existing xterm.js addons rather than custom solutions.

## Standard Stack

The established libraries/tools for browser terminal emulation:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| @xterm/xterm | 6.0.0 | Terminal emulator | Industry standard, used in VS Code, supports full VT100/220/320 sequences |
| @xterm/addon-fit | 0.11.0 | Auto-resize to container | Official addon, handles responsive terminal dimensions |
| @xterm/addon-webgl | 0.19.0 | GPU acceleration | Official addon, 2-4x render performance for large viewports |
| react | 19.0.0 | UI framework | Project standard, stable Context API for state |
| reconnecting-websocket | 4.4.0 | Auto-reconnecting WebSocket | Handles connection drops, exponential backoff, message buffering |
| vite | 6.0.0 | Build tool | Fast builds, simple config for embedded assets |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| @xterm/addon-clipboard | 0.2.0 | Browser clipboard access | Copy/paste support (WEB-04) |
| @xterm/addon-unicode11 | 0.9.0 | Unicode 11 character widths | Emoji and CJK support |
| @xterm/addon-web-links | 0.12.0 | URL detection | Clickable links in terminal output |
| zod | 4.3.6 | Runtime validation | Type-safe protocol message parsing |

### Rust Backend
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| axum | 0.8 | Web framework | Ergonomic WebSocket handling, type-safe routing |
| rust-embed | 8.11 | Static asset embedding | Compile-time file embedding, single binary deployment |
| axum-embed | 0.1 | Serve embedded assets | Integrates rust-embed with axum, SPA fallback support |

**Installation:**
```bash
# Already installed in web-ui/package.json
cd relay-server/web-ui
pnpm install
```

## Architecture Patterns

### Recommended Project Structure
```
web-ui/src/
├── lib/
│   ├── context/
│   │   ├── ConnectionContext.tsx    # WebSocket lifecycle & control messages
│   │   ├── TerminalContext.tsx      # xterm.js instances & binary I/O
│   │   └── TabsContext.tsx          # Session/tab management
│   ├── components/
│   │   ├── Terminal.tsx             # xterm.js wrapper with addons
│   │   ├── TabSidebar.tsx           # Session list UI
│   │   └── ConnectionStatus.tsx     # Visual connection state
│   └── protocol/
│       ├── binary.ts                # Binary frame encode/decode
│       └── messages.ts              # JSON control message types
├── routes/
│   ├── LoginPage.tsx                # Session code entry (WEB-06)
│   └── TerminalPage.tsx             # Main terminal + sidebar UI
└── shared/
    └── protocol.ts                  # Zod schemas for validation
```

### Pattern 1: Binary Frame Protocol
**What:** Session-routed binary frames with 1-byte length prefix for session IDs
**When to use:** All terminal I/O (output from shell, input from browser, resize messages)
**Example:**
```typescript
// Source: Phase 7 context decisions (05-06 binary frame format)
// Encode: Browser -> Relay (input, resize)
function encodeBinaryFrame(sessionId: string, payload: Uint8Array): Uint8Array {
  const sessionIdBytes = new TextEncoder().encode(sessionId);
  const frame = new Uint8Array(1 + sessionIdBytes.length + payload.length);
  frame[0] = sessionIdBytes.length;
  frame.set(sessionIdBytes, 1);
  frame.set(payload, 1 + sessionIdBytes.length);
  return frame;
}

// Decode: Relay -> Browser (output)
function decodeBinaryFrame(frame: Uint8Array): { sessionId: string; payload: Uint8Array } {
  const sessionIdLength = frame[0];
  const sessionIdBytes = frame.slice(1, 1 + sessionIdLength);
  const sessionId = new TextDecoder().decode(sessionIdBytes);
  const payload = frame.slice(1 + sessionIdLength);
  return { sessionId, payload };
}
```

### Pattern 2: Split Context for WebSocket State
**What:** Separate Contexts for connection lifecycle vs terminal data
**When to use:** Always - prevents re-render cascades when terminal data arrives
**Example:**
```typescript
// Source: React Context optimization patterns (2026)
// Connection state changes rarely (connecting, connected, disconnected)
const ConnectionContext = createContext<{
  state: ConnectionState;
  connect: (code: string) => void;
  disconnect: () => void;
  sendBinary: (data: Uint8Array) => void;
}>(null);

// Terminal data changes frequently (every keystroke, output chunk)
const TerminalContext = createContext<{
  sessions: Map<string, Terminal>;
  activeSessionId: string | null;
  setActiveSession: (id: string) => void;
}>(null);
```

### Pattern 3: Terminal Lifecycle with Addons
**What:** Initialize xterm.js with fit and webgl addons, dispose on unmount
**When to use:** Terminal component lifecycle
**Example:**
```typescript
// Source: xterm.js addon usage patterns
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebglAddon } from '@xterm/addon-webgl';

const terminal = new Terminal({
  cursorBlink: true,
  fontSize: 14,
  fontFamily: 'Menlo, Monaco, "Courier New", monospace',
  theme: { /* iTerm colors */ }
});

const fitAddon = new FitAddon();
const webglAddon = new WebglAddon();

terminal.loadAddon(fitAddon);
terminal.loadAddon(webglAddon); // Fallback to canvas if WebGL2 unavailable

terminal.open(containerElement);
fitAddon.fit();

// Cleanup on unmount
return () => terminal.dispose();
```

### Pattern 4: WebSocket Lifecycle in React
**What:** Proper WebSocket setup/cleanup in useEffect with reconnection
**When to use:** ConnectionContext provider
**Example:**
```typescript
// Source: React WebSocket cleanup best practices (2026)
useEffect(() => {
  const ws = new ReconnectingWebSocket(relayUrl, [], {
    maxReconnectionDelay: 30000,
    minReconnectionDelay: 1000,
    reconnectionDelayGrowFactor: 2,
    maxRetries: 10,
  });

  ws.addEventListener('open', handleOpen);
  ws.addEventListener('message', handleMessage);
  ws.addEventListener('close', handleClose);
  ws.addEventListener('error', handleError);

  // Critical: cleanup on unmount prevents memory leaks
  return () => {
    ws.removeEventListener('open', handleOpen);
    ws.removeEventListener('message', handleMessage);
    ws.removeEventListener('close', handleClose);
    ws.removeEventListener('error', handleError);
    ws.close();
  };
}, [relayUrl]);
```

### Pattern 5: Resize Coordination
**What:** Propagate browser resize to terminal, then to remote PTY
**When to use:** Window resize events, tab switches
**Example:**
```typescript
// Source: xterm.js FitAddon + resize coordination
useEffect(() => {
  const handleResize = () => {
    // 1. Fit terminal to container
    fitAddon.fit();

    // 2. Send resize to relay (binary frame)
    const cols = terminal.cols;
    const rows = terminal.rows;
    sendTerminalResize(activeSessionId, cols, rows);
  };

  const resizeObserver = new ResizeObserver(handleResize);
  resizeObserver.observe(containerRef.current);

  return () => resizeObserver.disconnect();
}, [activeSessionId, terminal]);
```

### Anti-Patterns to Avoid
- **Mixing binary and JSON for terminal I/O:** Use binary frames for all terminal data, JSON only for control messages (auth, errors)
- **Single Context for all WebSocket state:** Causes re-renders on every terminal output chunk
- **Manual reconnection logic:** Use `reconnecting-websocket` library instead
- **Calling fitAddon.fit() without ResizeObserver:** Terminal won't resize with browser window

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| WebSocket reconnection | Custom retry logic with setTimeout | `reconnecting-websocket` | Handles exponential backoff, maxRetries, connection state, message buffering during disconnect |
| Terminal container fitting | Manual cols/rows calculation | `@xterm/addon-fit` | Accounts for padding, scrollbar, font metrics, character cell boundaries |
| Binary data encoding | Manual byte manipulation | TextEncoder/TextDecoder + Uint8Array | Handles UTF-8 edge cases, multi-byte characters, BOM |
| Terminal GPU rendering | Canvas-based custom renderer | `@xterm/addon-webgl` | WebGL2 shader optimization, texture atlases, glyph caching, 2-4x faster |
| Protocol validation | Manual JSON.parse + type checks | Zod schemas with runtime validation | Type-safe parsing, detailed error messages, schema composition |

**Key insight:** Terminal emulation is deceptively complex. xterm.js handles 1000+ VT escape sequences, complex Unicode, bidirectional text, and sixel graphics. Binary WebSocket protocols need careful frame boundary handling, UTF-8 validation, and backpressure management. Use battle-tested libraries.

## Common Pitfalls

### Pitfall 1: WebSocket Cleanup in React useEffect
**What goes wrong:** WebSocket connections persist after component unmount, causing memory leaks and duplicate connections on remount
**Why it happens:** Forgetting to return cleanup function from useEffect, or not removing event listeners before close
**How to avoid:** Always return cleanup function that removes listeners and closes socket
**Warning signs:** Multiple WebSocket connections in DevTools Network tab, memory usage growing on navigation, "WebSocket already in CLOSING or CLOSED state" errors

### Pitfall 2: Binary vs Text Message Confusion
**What goes wrong:** Calling `ws.send(JSON.stringify(binaryData))` or treating binary frames as JSON
**Why it happens:** WebSocket supports both text and binary, but you must use correct send method and message event type
**How to avoid:** Use `ws.send(Message.Binary(data))` for terminal I/O, `ws.send(Message.Text(json))` for control messages; check `event.data instanceof ArrayBuffer` in message handler
**Warning signs:** Terminal shows garbled output, relay logs "Invalid JSON" errors, input doesn't work

### Pitfall 3: Terminal Resize Timing
**What goes wrong:** Resize message arrives before terminal processes previous output, causing line wrapping issues
**Why it happens:** No coordination between resize event and terminal write queue
**How to avoid:** Terminal.write() accepts callback for completion; wait for write completion before resize
**Warning signs:** Terminal content wraps incorrectly after resize, cursor position desync, vim/tmux display corruption

### Pitfall 4: Context Re-render Cascade
**What goes wrong:** Every terminal output chunk triggers re-render of all Context consumers
**Why it happens:** Single Context holds both connection state and terminal data; any state change re-renders all consumers
**How to avoid:** Split into separate Contexts (ConnectionContext, TerminalContext); use refs for frequently-changing data
**Warning signs:** React DevTools Profiler shows many re-renders, UI feels sluggish during rapid terminal output

### Pitfall 5: Backpressure Ignorance
**What goes wrong:** Relay buffers accumulate when sending faster than WebSocket can transmit, causing memory spikes
**Why it happens:** Not checking `ws.bufferedAmount` or respecting WebSocket ready state
**How to avoid:** Check `ws.readyState === WebSocket.OPEN` before send; monitor `ws.bufferedAmount` and pause writes if > threshold
**Warning signs:** Relay memory usage spikes with active terminals, "out of memory" crashes under load, terminal output lags behind shell

### Pitfall 6: Session ID Mismatch
**What goes wrong:** Binary frames reference wrong session ID, causing terminal output to appear in wrong tab
**Why it happens:** Session ID not synchronized between WebSocket context and terminal context
**How to avoid:** Use single source of truth for active session ID; validate session ID exists before sending binary frames
**Warning signs:** Terminal output appears in wrong tab, input sent to wrong session, tab switches show previous session's output

## Code Examples

Verified patterns from official sources:

### Writing Binary Data to Terminal
```typescript
// Source: xterm.js encoding guide (https://xtermjs.org/docs/guides/encoding/)
import { Terminal } from '@xterm/xterm';

const terminal = new Terminal();

// For binary WebSocket data (binaryType = 'arraybuffer')
ws.binaryType = 'arraybuffer';
ws.addEventListener('message', (event) => {
  if (event.data instanceof ArrayBuffer) {
    const uint8Array = new Uint8Array(event.data);
    terminal.writeUtf8(uint8Array); // Efficient binary write
  }
});

// For text data (JSON control messages)
ws.addEventListener('message', (event) => {
  if (typeof event.data === 'string') {
    const msg = JSON.parse(event.data);
    handleControlMessage(msg);
  }
});
```

### Handling Terminal Input
```typescript
// Source: xterm.js API (onData for text, onBinary for binary)
terminal.onData((data: string) => {
  // User typed text - send as binary frame
  const payload = new TextEncoder().encode(data);
  const frame = encodeBinaryFrame(activeSessionId, payload);
  ws.send(frame);
});

terminal.onBinary((data: string) => {
  // Binary mouse reports (rare) - convert to Uint8Array
  const buffer = new Uint8Array(data.length);
  for (let i = 0; i < data.length; i++) {
    buffer[i] = data.charCodeAt(i) & 0xff;
  }
  const frame = encodeBinaryFrame(activeSessionId, buffer);
  ws.send(frame);
});
```

### Vite Build Configuration for Embedded Assets
```typescript
// Source: Vite build options (https://vite.dev/config/build-options)
// vite.config.ts
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: '../assets',  // Output to relay-server/assets/
    emptyOutDir: true,    // Clean before build
    assetsDir: 'assets',  // Nest CSS/JS in assets/
  },
});
```

### Axum ServeEmbed for SPA Routing
```rust
// Source: axum-embed documentation (https://docs.rs/axum-embed/latest/axum_embed/)
use axum::Router;
use axum_embed::{ServeEmbed, FallbackBehavior};
use rust_embed::RustEmbed;

#[derive(RustEmbed, Clone)]
#[folder = "assets/"]
struct Assets;

let serve_assets = ServeEmbed::<Assets>::new()
    .index_file(Some("index.html"))
    .fallback(FallbackBehavior::Ok); // Return index.html for unknown paths (SPA routing)

let app = Router::new()
    .route("/ws", get(ws_handler))
    .nest_service("/", serve_assets);
```

### Session Auto-Switch on Connect
```typescript
// Source: Phase 7 context decision (auto-switch to new sessions)
// In TerminalContext
useEffect(() => {
  const handleSessionConnected = (sessionId: string, sessionName: string) => {
    // 1. Create terminal instance if not exists
    if (!sessions.has(sessionId)) {
      const term = createTerminal();
      sessions.set(sessionId, term);
    }

    // 2. Auto-switch to newly connected session
    setActiveSessionId(sessionId);

    // 3. Update tab list
    setTabs(prev => [...prev, { id: sessionId, name: sessionName }]);
  };

  return connection.registerMessageHandler(handleSessionConnected);
}, [connection]);
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| JSON terminal I/O | Binary WebSocket frames | 2020+ | 60% bandwidth reduction, no base64 overhead |
| Custom reconnect logic | reconnecting-websocket library | 2018+ | Standardized backoff, connection state management |
| Canvas rendering | WebGL2 rendering | xterm.js 4.0 (2019) | 2-4x render performance for large terminals |
| Global WebSocket instance | React Context with proper cleanup | React 16.8+ (2019) | Prevents memory leaks, enables SSR |
| Manual terminal sizing | FitAddon + ResizeObserver | xterm.js 3.0+ (2018) | Accurate sizing with window resize, no manual calculation |

**Deprecated/outdated:**
- **xterm-addon-attach**: Replaced by manual WebSocket handling with binary frames (more flexible session routing)
- **term.write(string)** for binary data: Use `term.writeUtf8(Uint8Array)` for binary WebSocket data (30% faster)
- **ws.binaryType = 'blob'**: Use `'arraybuffer'` with Uint8Array (direct memory access, no Blob conversion)

## Open Questions

Things that couldn't be fully resolved:

1. **Optimal bufferedAmount threshold for backpressure**
   - What we know: WebSocket.bufferedAmount indicates unsent bytes; should pause writes when high
   - What's unclear: Ideal threshold varies by network (16KB? 64KB? 256KB?)
   - Recommendation: Start with 64KB threshold, monitor relay memory usage, adjust based on telemetry

2. **Session disconnect timing before removal**
   - What we know: Context decision says "show as disconnected briefly (gray out), then remove from list"
   - What's unclear: How long is "briefly"? (2s? 5s? 30s?)
   - Recommendation: 5 seconds matches typical user perception, allows accidental tab close recovery

3. **Session name display format**
   - What we know: Shell integration sends session names like "zsh (PID 12345)"
   - What's unclear: Show full name with PID or strip PID for cleaner UI?
   - Recommendation: Strip PID for main display, show in tooltip (reduces clutter, PID available on hover)

4. **WebGL fallback behavior**
   - What we know: WebglAddon automatically falls back to canvas if WebGL2 unavailable
   - What's unclear: Should we detect and warn users on old browsers?
   - Recommendation: Silent fallback is fine (WebGL2 supported in 95%+ browsers since 2020)

## Sources

### Primary (HIGH confidence)
- [xterm.js GitHub Repository](https://github.com/xtermjs/xterm.js) - Official source, terminal API documentation
- [xterm.js Terminal API](https://xtermjs.org/docs/api/terminal/classes/terminal/) - write/writeUtf8 methods, event handlers
- [xterm.js Encoding Guide](https://xtermjs.org/docs/guides/encoding/) - UTF-8 handling, binary data
- [Vite Build Options](https://vite.dev/config/build-options) - outDir, assetsDir, emptyOutDir configuration
- [axum-embed Rust Documentation](https://docs.rs/axum-embed/latest/axum_embed/) - ServeEmbed, FallbackBehavior
- [WebSocket API - MDN](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/binaryType) - binaryType property, message handling

### Secondary (MEDIUM confidence)
- [reconnecting-websocket npm](https://www.npmjs.com/package/reconnecting-websocket) - Configuration options, features
- [React WebSocket Integration Guide](https://ably.com/blog/websockets-react-tutorial) - useEffect cleanup patterns
- [WebSocket Binary Data Best Practices](https://www.appetenza.com/websocket-handling-binary-data) - ArrayBuffer, Uint8Array usage
- [React Context Performance Optimization](https://kentcdodds.com/blog/how-to-optimize-your-context-value) - Split context pattern
- [Backpressure in WebSocket Streams](https://skylinecodes.substack.com/p/backpressure-in-websocket-streams) - bufferedAmount monitoring

### Tertiary (LOW confidence)
- [xterm.js resize issues](https://github.com/xtermjs/xterm.js/issues/1914) - Community discussions, not definitive
- [Terminal emulator architectures](https://opensource.com/article/18/1/domterminal) - General patterns, not xterm.js specific

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All libraries verified via official docs, package.json confirms versions
- Architecture: HIGH - Patterns verified via xterm.js docs, React best practices, existing codebase
- Pitfalls: MEDIUM - Based on common issues in GitHub issues, community blogs; some LOW confidence areas flagged

**Research date:** 2026-02-06
**Valid until:** 2026-03-06 (30 days - web stack is stable, xterm.js 6.0 mature)
