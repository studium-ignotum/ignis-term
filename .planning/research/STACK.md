# Technology Stack: Remote Terminal Control Web App

**Project:** Web-based remote control for iTerm2
**Researched:** 2026-02-04
**Research Method:** Training data (May 2025 cutoff) - web verification tools unavailable
**Overall Confidence:** MEDIUM (versions should be verified with `npm view [package] version`)

---

## Recommended Stack

### Terminal Emulation (CRITICAL)

| Technology | Version | Purpose | Confidence |
|------------|---------|---------|------------|
| xterm.js | ^5.x | Terminal emulation in browser | HIGH |
| @xterm/addon-fit | ^0.10.x | Auto-resize terminal to container | HIGH |
| @xterm/addon-webgl | ^0.18.x | GPU-accelerated rendering | HIGH |
| @xterm/addon-web-links | ^0.11.x | Clickable URLs in terminal | MEDIUM |
| @xterm/addon-canvas | ^0.7.x | Canvas fallback for non-WebGL | MEDIUM |

**Why xterm.js:**
- De facto standard for web terminal emulation (used by VS Code, Hyper, Theia)
- Full VT100/xterm compatibility including 256-color, true color, mouse events
- Active development, excellent performance with WebGL addon
- Mature addon ecosystem for common needs

**Why NOT alternatives:**
- `terminal.js` - abandoned, no longer maintained
- `hterm` (Chrome OS) - Google-internal focus, less community adoption
- `terminaljs` - limited features, no active development

### Frontend Framework

| Technology | Version | Purpose | Confidence |
|------------|---------|---------|------------|
| SvelteKit | ^2.x | Full-stack framework | HIGH |
| Svelte | ^5.x (runes) | Reactive UI | HIGH |
| TypeScript | ^5.x | Type safety | HIGH |

**Why SvelteKit:**
- Excellent reactivity model for real-time terminal state
- Svelte 5 runes (`$state`, `$derived`) perfect for terminal buffer management
- Minimal bundle size critical for responsive terminal feel
- Server-side flexibility for relay coordination
- Built-in adapter system for deployment (node, static, cloudflare, etc.)

**Why NOT alternatives:**
- `Next.js` - React's reconciliation overhead unnecessary for terminal rendering
- `Nuxt` - Vue's virtual DOM adds latency; xterm.js manages its own DOM
- `Remix` - Great for forms/data, but WebSocket story less mature

### WebSocket Layer

| Technology | Version | Purpose | Confidence |
|------------|---------|---------|------------|
| ws | ^8.x | Node.js WebSocket server | HIGH |

**Why `ws`:**
- Lightweight, zero-dependency WebSocket implementation
- Battle-tested (millions of downloads/week)
- Direct WebSocket protocol - no abstraction overhead
- Binary message support essential for terminal data
- Excellent backpressure handling for high-throughput scenarios

**Why NOT `socket.io`:**
- Socket.io adds unnecessary abstraction layer (polling fallback not needed in 2025)
- Higher latency due to protocol overhead
- Larger bundle size on client
- For terminal I/O, raw WebSocket is faster and simpler

**Why NOT `uWebSockets.js`:**
- Would be faster, but adds C++ dependency complexity
- `ws` is fast enough for terminal relay (not millions of concurrent connections)
- Better ecosystem integration with Node.js

### Server Runtime

| Technology | Version | Purpose | Confidence |
|------------|---------|---------|------------|
| Node.js | ^22.x LTS | Server runtime | HIGH |
| node-pty | ^1.x | PTY for local terminal | HIGH (Mac side) |

**Why Node.js 22:**
- LTS version with long-term support
- Native WebSocket in `http` module improvements
- Better ES module support
- Performance improvements for streaming

**Why `node-pty`:**
- Required on Mac client side to interface with iTerm2 sessions
- Provides proper PTY (pseudo-terminal) handling
- Handles terminal sizing, signals correctly

### Authentication

| Technology | Purpose | Confidence |
|------------|---------|------------|
| nanoid | Session code generation | HIGH |
| Secure random codes | 6-8 character session codes | HIGH |

**Why simple session codes over OAuth/JWT:**
- Use case is ephemeral pairing (like AirDrop)
- No persistent accounts needed
- Session codes expire after connection established
- Simpler UX: "Enter code: ABC123" vs OAuth flow

**Session code pattern:**
```typescript
import { nanoid, customAlphabet } from 'nanoid';

// Alphanumeric, no ambiguous chars (0/O, 1/I/l)
const generateSessionCode = customAlphabet('23456789ABCDEFGHJKLMNPQRSTUVWXYZ', 6);
```

### Relay Server Infrastructure

| Technology | Purpose | Confidence |
|------------|---------|------------|
| Fly.io or Railway | Edge deployment | MEDIUM |
| Redis (optional) | Session state if multi-instance | MEDIUM |

**Architecture pattern:**
```
[Mac/iTerm2] <--WebSocket--> [Cloud Relay] <--WebSocket--> [Browser]
     |                            |                            |
  node-pty                   ws server                     xterm.js
```

**Why cloud relay:**
- NAT traversal without port forwarding
- Works from any network
- Session codes for discovery
- No direct IP exposure

---

## Supporting Libraries

| Library | Version | Purpose | When to Use | Confidence |
|---------|---------|---------|-------------|------------|
| zod | ^3.x | Schema validation | Session messages | HIGH |
| nanoid | ^5.x | ID generation | Session codes | HIGH |
| mitt | ^3.x | Event emitter | Client-side events | MEDIUM |
| reconnecting-websocket | ^4.x | Auto-reconnect | Browser WebSocket | HIGH |

---

## Development Tools

| Tool | Purpose | Confidence |
|------|---------|------------|
| Vite | Build tool (via SvelteKit) | HIGH |
| Vitest | Unit testing | HIGH |
| Playwright | E2E testing | HIGH |
| ESLint + Prettier | Code quality | HIGH |

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not Alternative |
|----------|-------------|-------------|---------------------|
| Terminal | xterm.js | hterm | Less community, Google-focused |
| Framework | SvelteKit | Next.js | React overhead for terminal rendering |
| WebSocket | ws | socket.io | Unnecessary abstraction, higher latency |
| WebSocket | ws | uWebSockets.js | C++ complexity not worth marginal perf gain |
| Auth | Session codes | JWT | Overkill for ephemeral pairing |

---

## Installation

```bash
# Frontend (SvelteKit app)
npm create svelte@latest remote-terminal
cd remote-terminal
npm install xterm @xterm/addon-fit @xterm/addon-webgl
npm install -D typescript @types/node

# Relay server dependencies
npm install ws nanoid zod
npm install -D @types/ws

# Mac client dependencies (separate package)
npm install ws node-pty nanoid
```

---

## Version Verification Required

**IMPORTANT:** The following versions should be verified before implementation:

```bash
# Run these to get current versions
npm view xterm version
npm view @xterm/addon-fit version
npm view @xterm/addon-webgl version
npm view ws version
npm view @sveltejs/kit version
npm view nanoid version
npm view zod version
npm view node-pty version
```

My training data has a May 2025 cutoff. Libraries may have released newer versions with breaking changes.

---

## Key Implementation Notes

### xterm.js Setup Pattern

```typescript
import { Terminal } from 'xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebglAddon } from '@xterm/addon-webgl';
import 'xterm/css/xterm.css';

const terminal = new Terminal({
  cursorBlink: true,
  fontSize: 14,
  fontFamily: 'Menlo, Monaco, "Courier New", monospace',
  theme: {
    background: '#1e1e1e',
    foreground: '#d4d4d4',
  },
});

const fitAddon = new FitAddon();
terminal.loadAddon(fitAddon);

// Load WebGL after terminal opens for performance
terminal.open(containerElement);
terminal.loadAddon(new WebglAddon());
fitAddon.fit();

// Handle resize
window.addEventListener('resize', () => fitAddon.fit());
```

### WebSocket Binary Protocol

For terminal I/O, use binary messages (not JSON) for performance:

```typescript
// Server (relay)
wss.on('connection', (ws) => {
  ws.binaryType = 'arraybuffer';

  ws.on('message', (data, isBinary) => {
    if (isBinary) {
      // Forward terminal data as-is
      targetWs.send(data, { binary: true });
    } else {
      // Control messages as JSON
      const msg = JSON.parse(data.toString());
      handleControlMessage(msg);
    }
  });
});

// Client (browser)
socket.binaryType = 'arraybuffer';
socket.onmessage = (event) => {
  if (event.data instanceof ArrayBuffer) {
    terminal.write(new Uint8Array(event.data));
  } else {
    handleControlMessage(JSON.parse(event.data));
  }
};
```

### SvelteKit WebSocket Integration

SvelteKit doesn't have built-in WebSocket support. Use a custom server:

```typescript
// vite.config.ts - for development
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { WebSocketServer } from 'ws';

export default defineConfig({
  plugins: [
    sveltekit(),
    {
      name: 'websocket',
      configureServer(server) {
        const wss = new WebSocketServer({ noServer: true });
        server.httpServer?.on('upgrade', (req, socket, head) => {
          if (req.url?.startsWith('/ws')) {
            wss.handleUpgrade(req, socket, head, (ws) => {
              wss.emit('connection', ws, req);
            });
          }
        });
      },
    },
  ],
});
```

For production, use `adapter-node` and handle WebSocket upgrade in custom server entry.

---

## Sources

- xterm.js: Training data knowledge of npm package and VS Code usage
- ws: Training data knowledge of npm package ecosystem
- SvelteKit: Training data knowledge of Svelte 5 and SvelteKit 2
- node-pty: Training data knowledge of terminal integration patterns

**Confidence caveat:** All version numbers and some API details come from training data with May 2025 cutoff. Verify current versions before implementation.

---

## Roadmap Implications

1. **Phase 1 - Core Infrastructure:** Set up SvelteKit + xterm.js + ws relay. This is well-documented territory.

2. **Phase 2 - iTerm2 Integration:** node-pty integration on Mac side. May need research on iTerm2-specific APIs if going beyond basic PTY.

3. **Phase 3 - Session Management:** Session codes, reconnection handling. Standard patterns apply.

4. **Phase 4 - Multi-Tab:** Tab switching, session listing. iTerm2 AppleScript/API research may be needed.

**Research flags:**
- iTerm2 scripting API (for tab listing) - needs phase-specific research
- Deployment options (Fly.io vs Railway vs Cloudflare) - needs cost/latency comparison
