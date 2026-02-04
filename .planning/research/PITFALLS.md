# Domain Pitfalls: Remote Terminal Control Web App

**Domain:** Web-based remote terminal (Mac client -> Cloud Relay <- Browser)
**Stack:** xterm.js, SvelteKit, Node.js, WebSocket
**Researched:** 2026-02-04
**Confidence:** MEDIUM (training knowledge, web research unavailable)

---

## Critical Pitfalls

Mistakes that cause rewrites, security vulnerabilities, or major reliability issues.

---

### Pitfall 1: WebSocket Reconnection Without State Synchronization

**What goes wrong:** After a connection drop, the client reconnects but the terminal state is desynchronized. User sees corrupted output, missing characters, or stale content. Particularly bad when the drop happens mid-escape-sequence.

**Why it happens:** Developers implement reconnection logic but don't handle the terminal buffer state. The relay may have buffered data that was sent but never acknowledged. The terminal cursor position, scroll region, and character attributes are lost.

**Consequences:**
- Garbled terminal output requiring manual refresh
- Lost command output (user thinks command didn't run)
- Cursor in wrong position, input appears in wrong place
- Escape sequences split across reconnection cause rendering artifacts

**Warning signs:**
- Users report "weird characters" after wifi blip
- Terminal "fixes itself" after running `clear`
- Intermittent reports of "stuck" terminals

**Prevention:**
1. Implement sequence-numbered messages (relay tracks what browser acknowledged)
2. On reconnect, replay unacknowledged data
3. Consider periodic terminal state snapshots (cursor pos, scroll region, character attrs)
4. Send a terminal reset sequence on reconnect if state is unrecoverable
5. Use xterm.js `serialize` addon to capture/restore terminal state

**Detection code pattern:**
```typescript
// Track message acknowledgment
interface Message {
  seq: number;
  data: string;
  acknowledged: boolean;
}

// On reconnect
const unacked = messages.filter(m => !m.acknowledged);
for (const msg of unacked) {
  terminal.write(msg.data);
}
```

**Phase:** Address in Phase 1 (Connection/Infrastructure) - this is foundational

---

### Pitfall 2: Terminal Resize Race Conditions

**What goes wrong:** User resizes browser window, terminal dimensions update locally, but the PTY on the Mac doesn't resize in sync. Output wraps incorrectly. Full-screen apps like vim/tmux render garbage.

**Why it happens:** Resize events fire rapidly during drag. Network latency means PTY resize arrives after data was already rendered for old dimensions. No coordination between terminal render size and PTY knowledge of size.

**Consequences:**
- vim/nano/tmux become unusable after resize
- Command output wraps at wrong column
- ncurses apps show corrupted UI
- Users have to disconnect/reconnect to fix

**Warning signs:**
- Resize works on first try but breaks on rapid resizing
- Works locally but breaks over slow connections
- vim users report frequent display corruption

**Prevention:**
1. Debounce resize events (200-300ms typically)
2. Send resize with sequence number, don't apply locally until confirmed
3. After resize confirmation, request full redraw from PTY (`SIGWINCH` sends redraw for most apps)
4. Consider resize locking: show "resizing..." overlay, unlock after confirmation
5. Store and validate dimensions match on both ends

**Detection:**
```typescript
// Debounce + confirmation pattern
let resizeTimeout: NodeJS.Timeout;
let pendingResize: { cols: number; rows: number } | null = null;

terminal.onResize(({ cols, rows }) => {
  clearTimeout(resizeTimeout);
  pendingResize = { cols, rows };
  resizeTimeout = setTimeout(() => {
    sendResize(pendingResize);
    // Don't apply to fitAddon until server confirms
  }, 250);
});

socket.on('resize-ack', ({ cols, rows }) => {
  if (pendingResize?.cols === cols && pendingResize?.rows === rows) {
    fitAddon.fit(); // Now safe to apply
    pendingResize = null;
  }
});
```

**Phase:** Address in Phase 2 (Terminal Rendering) - after basic connection works

---

### Pitfall 3: Authentication Token Exposure in WebSocket URL

**What goes wrong:** Auth token passed as query parameter in WebSocket URL (`wss://relay.com?token=xxx`). Token appears in server logs, browser history, referrer headers, and any proxy logs.

**Why it happens:** WebSocket API doesn't support custom headers in browser. Developers take the "easy" path of query params. Works functionally but creates security vulnerability.

**Consequences:**
- Tokens in server access logs (often stored long-term)
- Tokens visible in browser dev tools network tab (shoulder surfing)
- Tokens in proxy logs if corporate network
- Replay attacks possible if tokens long-lived

**Warning signs:**
- Security audit flags it immediately
- Tokens appearing in error tracking/logging tools
- Users report sessions being "hijacked" (rare but devastating)

**Prevention:**
1. Use short-lived connection tickets: HTTP request gets single-use ticket, WebSocket connects with ticket (valid for 30 seconds, single use)
2. Implement proper WebSocket authentication handshake after connection
3. Never log query parameters containing tokens
4. Rotate connection credentials frequently
5. Consider mTLS for Mac client to relay connection

**Secure pattern:**
```typescript
// Step 1: Get short-lived ticket via authenticated HTTP
const { ticket } = await fetch('/api/ws-ticket', {
  headers: { Authorization: `Bearer ${authToken}` }
}).then(r => r.json());

// Step 2: Connect with single-use ticket (30 sec validity)
const ws = new WebSocket(`wss://relay.com/connect?ticket=${ticket}`);

// Step 3: Ticket validated and invalidated on server, real session established
ws.onopen = () => {
  // Connection authenticated, ticket can never be reused
};
```

**Phase:** Address in Phase 1 (Auth/Security) - before any other features

---

### Pitfall 4: Memory Leaks from Terminal Buffer Growth

**What goes wrong:** Terminal scrollback buffer grows unbounded. Browser tab consumes gigabytes of memory over time. Eventually crashes or becomes unresponsive.

**Why it happens:** xterm.js default scrollback is 1000 lines but can be set higher. More importantly, developers often keep parallel data structures (for search, logging, replay) that also grow unbounded.

**Consequences:**
- Browser tab crashes after hours of use
- Gradual performance degradation
- Mobile browsers especially affected
- Users blame the app for "being slow"

**Warning signs:**
- Memory usage grows linearly with session duration
- Performance degrades over long sessions
- Mobile users report crashes more than desktop

**Prevention:**
1. Set explicit scrollback limit: `new Terminal({ scrollback: 5000 })`
2. Audit all parallel data structures for the same limit
3. Implement circular buffers for any logging/replay features
4. Add memory monitoring and warning at threshold
5. Consider "trim" feature for very long sessions

**Configuration:**
```typescript
const terminal = new Terminal({
  scrollback: 5000, // Explicit limit
  // ... other options
});

// If you maintain parallel structures:
class BoundedBuffer<T> {
  private buffer: T[] = [];
  private maxSize: number;

  constructor(maxSize: number) {
    this.maxSize = maxSize;
  }

  push(item: T) {
    this.buffer.push(item);
    if (this.buffer.length > this.maxSize) {
      this.buffer.shift(); // Remove oldest
    }
  }
}
```

**Phase:** Address in Phase 2 (Terminal Rendering) - configure early, monitor ongoing

---

### Pitfall 5: Relay Connection State Mismatch (Three-Party Problem)

**What goes wrong:** Browser thinks it's connected, relay thinks it's connected, but Mac client has actually disconnected. User types commands that go nowhere. Or: Mac client disconnected from PTY but relay doesn't know.

**Why it happens:** Three-party architecture (Browser <-> Relay <-> Mac) has more failure modes than two-party. Each connection can fail independently. Without end-to-end health checks, intermediate state becomes stale.

**Consequences:**
- User types into void (feels broken, no feedback)
- "Connected" indicator lies to user
- Commands silently lost
- Delayed error feedback (might take 30+ seconds to realize)

**Warning signs:**
- Users report "it just stopped working"
- Works again after page refresh
- Logs show connections but no data flow
- Intermittent reports impossible to reproduce

**Prevention:**
1. End-to-end heartbeat: Browser sends ping, must reach Mac and return (not just relay)
2. Relay immediately notifies browser when Mac disconnects
3. Display connection state for BOTH legs (Browser-Relay and Relay-Mac)
4. Implement "connection quality" indicator (latency, recent success rate)
5. Auto-reconnect with exponential backoff when Mac side drops

**State machine:**
```typescript
type ConnectionState =
  | 'disconnected'           // No connection
  | 'connecting'             // Establishing
  | 'relay-connected'        // Browser <-> Relay OK, Mac unknown
  | 'fully-connected'        // Both legs confirmed
  | 'mac-disconnected'       // Relay connected but Mac dropped
  | 'reconnecting';          // Attempting to restore

// Display different UI for each state
// Only allow input when 'fully-connected'
```

**Phase:** Address in Phase 1 (Connection/Infrastructure) - core architecture decision

---

### Pitfall 6: Input Handling for Special Keys and Modifiers

**What goes wrong:** Ctrl+C doesn't send interrupt. Ctrl+D doesn't EOF. Arrow keys send garbage in some terminals. Meta/Alt key combinations fail. Copy/paste conflicts with terminal selection.

**Why it happens:** Keyboard handling is complex: browser events, xterm.js processing, encoding to PTY, and shell interpretation all must align. Platform differences (Mac Cmd vs Win Ctrl) add complexity.

**Consequences:**
- Can't cancel running processes (Ctrl+C)
- Can't exit programs properly (Ctrl+D)
- Command line editing broken (arrows, home/end)
- Users can't use keyboard shortcuts they rely on
- Copy/paste doesn't work as expected

**Warning signs:**
- Users report Ctrl+C "doesn't work"
- Arrow keys show `^[[A` instead of moving cursor
- Different behavior across browsers
- Mac vs Windows users have different experiences

**Prevention:**
1. Test ALL common key combinations explicitly (create test matrix)
2. Use xterm.js attachCustomKeyEventHandler for browser shortcuts that conflict
3. Handle Mac Cmd key appropriately (Cmd+C for copy, Ctrl+C for SIGINT)
4. Test in multiple browsers (Chrome, Firefox, Safari have differences)
5. Provide keyboard shortcut documentation/reference

**Test matrix (must all work):**
```
| Key Combo      | Expected Action                    |
|----------------|------------------------------------|
| Ctrl+C         | SIGINT (0x03)                      |
| Ctrl+D         | EOF (0x04)                         |
| Ctrl+Z         | SIGTSTP (0x1A)                     |
| Ctrl+L         | Clear screen (0x0C)                |
| Arrow keys     | Cursor movement (escape sequences) |
| Home/End       | Line navigation                    |
| Ctrl+Arrow     | Word navigation                    |
| Ctrl+A/E       | Line start/end (emacs mode)        |
| Tab            | Completion                         |
| Ctrl+R         | Reverse search                     |
| Cmd+C (Mac)    | Copy selection (NOT SIGINT)        |
| Cmd+V (Mac)    | Paste                              |
```

**Phase:** Address in Phase 2 (Terminal Rendering) - test thoroughly before UX polish

---

## Moderate Pitfalls

Mistakes that cause delays, technical debt, or degraded user experience.

---

### Pitfall 7: WebGL Renderer Compatibility Issues

**What goes wrong:** xterm.js WebGL renderer (faster) fails on some devices/browsers. Fallback to canvas renderer not implemented. Terminal shows black screen or crashes.

**Why it happens:** WebGL has varying support across devices, especially: older machines, VMs, remote desktops, some mobile browsers, privacy-focused browser configs that disable WebGL.

**Consequences:**
- Terminal unusable for subset of users
- Hard to debug (works on dev machines)
- Black screen with no error message
- Users think app is broken

**Prevention:**
1. Always implement canvas fallback
2. Detect WebGL failure and switch automatically
3. Let users manually choose renderer in settings
4. Log which renderer is active for debugging
5. Test in VM and with WebGL disabled

**Implementation:**
```typescript
import { Terminal } from 'xterm';
import { WebglAddon } from 'xterm-addon-webgl';
import { CanvasAddon } from 'xterm-addon-canvas';

const terminal = new Terminal();

try {
  const webglAddon = new WebglAddon();
  webglAddon.onContextLoss(() => {
    webglAddon.dispose();
    loadCanvasFallback();
  });
  terminal.loadAddon(webglAddon);
  console.log('Using WebGL renderer');
} catch (e) {
  console.log('WebGL unavailable, using canvas');
  loadCanvasFallback();
}

function loadCanvasFallback() {
  terminal.loadAddon(new CanvasAddon());
}
```

**Phase:** Address in Phase 2 (Terminal Rendering) - implement fallback from start

---

### Pitfall 8: No Offline/Disconnected State Handling

**What goes wrong:** Network drops, app shows spinner forever or fails silently. No indication of what's happening. No way to reconnect without full page reload.

**Why it happens:** Happy path development - everything works with good connectivity. Error states and offline handling are "edge cases" that get deferred.

**Consequences:**
- Users don't know if they're connected
- Lost work when connection drops
- Manual refresh needed to recover
- App feels unreliable

**Prevention:**
1. Show clear connection status indicator always visible
2. Queue keystrokes during brief disconnections (with limit)
3. Implement automatic reconnection with backoff
4. Show reconnection progress and allow cancel
5. Preserve terminal state across reconnections (see Pitfall 1)

**UX patterns:**
```
States to handle:
- Connecting... (show progress)
- Connected (subtle indicator)
- Connection lost, reconnecting... (prominent, show attempt count)
- Reconnection failed (show error, retry button)
- Offline mode (if supported)

Keystroke queue:
- Buffer up to 1KB of input during disconnect
- Show "buffered input" indicator
- Send on reconnect
- Clear if reconnect fails (user must retype)
```

**Phase:** Address in Phase 3 (UI/UX Polish) - after core connection works

---

### Pitfall 9: Blocking Operations on WebSocket Message Handler

**What goes wrong:** Heavy processing in WebSocket message handler blocks the event loop. Terminal becomes unresponsive during large output (e.g., `cat large_file.txt`).

**Why it happens:** All terminal data processing happens synchronously in the message handler. Large bursts of data freeze the UI.

**Consequences:**
- Terminal freezes during large output
- Input blocked while processing output
- Browser may show "page unresponsive" warning
- Particularly bad with continuous output (logs, builds)

**Prevention:**
1. Chunk large data writes with requestAnimationFrame
2. Use xterm.js flow control (it has built-in support)
3. Consider Web Worker for data processing
4. Implement backpressure to relay (pause/resume)
5. Profile with large output scenarios

**Chunking pattern:**
```typescript
const CHUNK_SIZE = 16384; // 16KB chunks
let writeQueue: string[] = [];
let isWriting = false;

function queueWrite(data: string) {
  writeQueue.push(data);
  if (!isWriting) {
    processQueue();
  }
}

function processQueue() {
  if (writeQueue.length === 0) {
    isWriting = false;
    return;
  }

  isWriting = true;
  const chunk = writeQueue.shift()!;

  if (chunk.length > CHUNK_SIZE) {
    writeQueue.unshift(chunk.slice(CHUNK_SIZE));
    terminal.write(chunk.slice(0, CHUNK_SIZE));
  } else {
    terminal.write(chunk);
  }

  requestAnimationFrame(processQueue);
}
```

**Phase:** Address in Phase 2 (Terminal Rendering) - test with large output

---

### Pitfall 10: Incorrect Character Encoding Handling

**What goes wrong:** Non-ASCII characters display as garbage. Emoji broken. International characters fail. Or: binary data in terminal causes corruption.

**Why it happens:** Encoding mismatches between: shell locale, PTY settings, WebSocket message encoding, xterm.js expectations. Usually assumes UTF-8 everywhere but one component doesn't comply.

**Consequences:**
- Non-English users can't use the terminal
- Filenames with special characters show incorrectly
- Git diff with emoji shows garbage
- Programming languages with unicode (Python, JS) break

**Prevention:**
1. Ensure UTF-8 throughout: PTY, shell, relay, WebSocket all configured UTF-8
2. Set `LANG` and `LC_ALL` environment variables on PTY
3. Use binary WebSocket frames (not text) to avoid encoding issues
4. Handle binary data separately (don't feed to terminal as-is)
5. Test with emoji, CJK characters, RTL text

**Environment setup:**
```typescript
// When spawning PTY on Mac client
const pty = spawn(shell, [], {
  env: {
    ...process.env,
    LANG: 'en_US.UTF-8',
    LC_ALL: 'en_US.UTF-8',
    TERM: 'xterm-256color',
  },
  encoding: 'utf8',
});

// WebSocket: use binary frames
ws.binaryType = 'arraybuffer';

// Decode properly on receive
ws.onmessage = (event) => {
  const data = new TextDecoder('utf-8').decode(event.data);
  terminal.write(data);
};
```

**Phase:** Address in Phase 1 (Infrastructure) - configure correctly from start

---

## Minor Pitfalls

Issues that cause annoyance but are readily fixable.

---

### Pitfall 11: Terminal Cursor Style Not Matching User Preference

**What goes wrong:** Cursor is block when user prefers line. Blink behavior doesn't match local terminal. Feels "off" to power users.

**Prevention:** Make cursor style configurable, sync with common terminal preferences.

**Phase:** Phase 3 (UI/UX Polish)

---

### Pitfall 12: No Visual Bell Support

**What goes wrong:** Programs that use bell for alerts don't notify user. User misses important events.

**Prevention:** Implement visual bell (flash) and optional audio. Make configurable.

**Phase:** Phase 3 (UI/UX Polish)

---

### Pitfall 13: Selection/Copy Doesn't Include Full Scrollback

**What goes wrong:** User can only copy visible text, not text scrolled off screen.

**Prevention:** Use xterm.js selection manager properly, allow selecting into scrollback.

**Phase:** Phase 2 (Terminal Rendering)

---

### Pitfall 14: No Session Persistence/Reconnection

**What goes wrong:** Browser refresh loses everything. Tab accidentally closed = session gone.

**Prevention:** Implement session persistence on relay, reconnect to existing PTY on page load if available.

**Phase:** Phase 3 (Advanced Features) - nice to have

---

### Pitfall 15: Font Loading Flash

**What goes wrong:** Terminal briefly shows in wrong font, then re-renders. Looks janky.

**Prevention:** Load monospace font before creating terminal. Use font loading API.

**Phase:** Phase 2 (Terminal Rendering)

---

## Phase-Specific Warning Summary

| Phase | Topic | Likely Pitfalls | Priority |
|-------|-------|-----------------|----------|
| Phase 1 | Connection Infrastructure | #1 (State sync), #5 (Three-party), #3 (Auth tokens), #10 (Encoding) | CRITICAL |
| Phase 2 | Terminal Rendering | #2 (Resize), #4 (Memory), #6 (Input), #7 (WebGL), #9 (Blocking) | HIGH |
| Phase 3 | UI/UX Polish | #8 (Offline handling), #11-15 (Minor) | MEDIUM |
| Phase 4 | Hardening | Revisit all, load testing, security audit | HIGH |

---

## Pre-Implementation Checklist

Before building each component, verify:

**Connection Layer:**
- [ ] End-to-end heartbeat designed (not just relay ping)
- [ ] Reconnection includes state recovery plan
- [ ] Auth uses short-lived tickets, not URL tokens
- [ ] Three-party state machine documented

**Terminal Layer:**
- [ ] Scrollback limit set explicitly
- [ ] WebGL fallback to canvas implemented
- [ ] Resize debounced with confirmation
- [ ] Key combo test matrix created

**UX Layer:**
- [ ] Connection state visible to user
- [ ] Error states have recovery actions
- [ ] Disconnection doesn't lose buffered input
- [ ] Works on mobile browsers

---

## Sources

- Training knowledge of xterm.js common issues (MEDIUM confidence)
- Domain experience with WebSocket architecture patterns (MEDIUM confidence)
- Terminal emulation fundamentals (HIGH confidence - stable domain)

**Note:** WebSearch and WebFetch were unavailable during research. Recommendations should be validated against current xterm.js documentation and community discussions before implementation.
