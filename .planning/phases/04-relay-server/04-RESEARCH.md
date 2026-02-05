# Phase 4: Relay Server - Research

**Researched:** 2026-02-06
**Domain:** Rust WebSocket relay server with embedded static assets
**Confidence:** HIGH

## Summary

This research extends the existing project research (in `.planning/research/`) with implementation-specific details for the relay server. The relay server is the first component of the v2.0 Rust rewrite and has no dependencies on other components, making it an ideal starting point.

The relay server must handle three concerns: (1) WebSocket connections from mac-clients and browsers, (2) session code generation and validation for pairing clients with browsers, and (3) static file serving for the embedded web UI. All three are well-supported by the established stack: axum 0.8 for HTTP/WebSocket, rust-embed with axum-embed for static assets, and nanoid for session codes.

The core pattern is straightforward: mac-client connects via WebSocket and receives a session code, browser connects with that code and gets paired, messages route between the paired connections. State management uses `Arc<AppState>` with `DashMap` for concurrent access and `tokio-util::DelayQueue` for session expiration.

**Primary recommendation:** Build a minimal viable relay with single mac-client per session code first, then add multi-session support. The axum chat example provides the exact pattern needed for broadcast-style message routing.

## Standard Stack

The established libraries/tools for this domain (versions verified in existing research):

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| axum | ^0.8 | HTTP/WebSocket framework | Tokio-team maintained, native WebSocket via `extract::ws`, superior DX |
| tokio | ^1.43 | Async runtime | De facto standard, required by axum |
| rust-embed | ^8.11 | Compile-time asset embedding | Debug/release dual behavior, compression support |
| axum-embed | ^0.2 | Serve rust-embed with axum | Turnkey integration, ETag support, SPA fallback |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tower-http | ^0.6 | HTTP middleware | CORS, compression, request tracing |
| nanoid | ^0.4 | Session code generation | Short, secure, URL-friendly IDs |
| dashmap | ^6 | Concurrent HashMap | Multi-client state without Mutex contention |
| tokio-util | ^0.7 | DelayQueue for TTL | Session expiration with automatic cleanup |
| serde | ^1 | Serialization | JSON control messages |
| serde_json | ^1 | JSON parsing | Control message encoding/decoding |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| dashmap | Arc<RwLock<HashMap>> | DashMap has better concurrent access, less lock contention |
| nanoid | uuid | nanoid is shorter (6 chars vs 36), more user-friendly for session codes |
| axum-embed | tower-http ServeDir | ServeDir requires filesystem, we want single binary |
| tokio-util DelayQueue | Manual timer tasks | DelayQueue handles concurrent expirations correctly |

**Cargo.toml dependencies:**
```toml
[dependencies]
axum = { version = "0.8", features = ["ws"] }
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.6", features = ["cors", "trace"] }
rust-embed = { version = "8.11", features = ["compression"] }
axum-embed = "0.2"
nanoid = "0.4"
dashmap = "6"
tokio-util = { version = "0.7", features = ["time"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
```

## Architecture Patterns

### Recommended Project Structure
```
relay-server/
├── src/
│   ├── main.rs           # Entry point, server setup
│   ├── state.rs          # AppState, session management
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── ws.rs         # WebSocket upgrade and routing
│   │   └── static_files.rs  # Embedded asset serving
│   ├── protocol.rs       # Message types, serialization
│   └── session.rs        # Session code generation, expiration
├── web-ui/               # Frontend assets (xterm.js UI)
│   └── dist/             # Built assets to embed
└── Cargo.toml
```

### Pattern 1: WebSocket Handler with Client Differentiation

**What:** Single WebSocket endpoint that differentiates client types by first message
**When to use:** When mac-clients and browsers connect to the same endpoint

```rust
// Source: https://github.com/tokio-rs/axum/blob/main/examples/websockets/src/main.rs
use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade, Message}, State},
    response::IntoResponse,
};

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    // First message determines client type
    let Some(Ok(first_msg)) = socket.recv().await else {
        return;
    };

    match parse_client_type(&first_msg) {
        ClientType::MacClient => handle_mac_client(socket, state).await,
        ClientType::Browser { session_code } => {
            handle_browser(socket, session_code, state).await
        }
    }
}
```

### Pattern 2: Concurrent Send/Receive with Socket Split

**What:** Split WebSocket into sender and receiver for independent task handling
**When to use:** When you need to send messages while simultaneously receiving

```rust
// Source: https://docs.rs/axum/latest/axum/extract/ws/index.html
use futures_util::{SinkExt, StreamExt};

async fn handle_mac_client(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // Generate session code and register
    let session_code = generate_session_code();
    state.register_mac_client(session_code.clone(), sender).await;

    // Send registration confirmation
    let response = json!({ "type": "registered", "code": session_code });
    // sender is now owned by state, use channel to send

    // Process incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        state.route_message(&session_code, msg).await;
    }

    // Cleanup on disconnect
    state.unregister_mac_client(&session_code).await;
}
```

### Pattern 3: Broadcast Channel for Multi-Client Updates

**What:** Use tokio broadcast channel when one mac-client has multiple browser viewers
**When to use:** When terminal output should reach all connected browsers

```rust
// Source: https://github.com/tokio-rs/axum/blob/main/examples/chat/src/main.rs
use tokio::sync::broadcast;

struct Session {
    mac_client_tx: mpsc::Sender<Message>,  // Send to mac-client
    browser_tx: broadcast::Sender<Vec<u8>>, // Broadcast to all browsers
}

impl Session {
    fn new() -> Self {
        let (browser_tx, _) = broadcast::channel(100);
        // mac_client_tx set when mac-client connects
        Self { browser_tx, mac_client_tx: /* ... */ }
    }

    // Terminal output from mac-client -> all browsers
    async fn broadcast_output(&self, data: Vec<u8>) {
        let _ = self.browser_tx.send(data);  // Ignore if no receivers
    }
}
```

### Pattern 4: State Management with DashMap

**What:** Use DashMap for concurrent access without explicit locking
**When to use:** When multiple WebSocket tasks access shared session state

```rust
// Source: https://docs.rs/dashmap/latest/dashmap/struct.DashMap.html
use dashmap::DashMap;

struct AppState {
    // Session code -> Session data
    sessions: DashMap<String, Session>,
    // Session code -> Vec of browser senders (for broadcast)
    browsers: DashMap<String, Vec<mpsc::Sender<Message>>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            browsers: DashMap::new(),
        }
    }

    async fn register_browser(&self, code: &str, tx: mpsc::Sender<Message>) -> bool {
        if self.sessions.contains_key(code) {
            self.browsers
                .entry(code.to_string())
                .or_default()
                .push(tx);
            true
        } else {
            false  // Invalid session code
        }
    }
}
```

### Anti-Patterns to Avoid

- **Single Mutex for all state:** Creates contention bottleneck. Use DashMap or separate locks per session.
- **Blocking in async handlers:** Never do CPU-intensive work in WebSocket handlers. Use `spawn_blocking` if needed.
- **Unbounded channels:** Always use bounded channels to prevent memory exhaustion from slow consumers.
- **Ignoring close frames:** Always handle WebSocket Close messages gracefully for clean disconnection.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Session code generation | Random string function | nanoid | Cryptographically secure, proper distribution |
| Concurrent HashMap | Arc<Mutex<HashMap>> | DashMap | Better performance, less boilerplate |
| Session expiration | Manual timer per session | tokio-util DelayQueue | Handles concurrent expirations, automatic cleanup |
| Static file serving | Custom handler | axum-embed | ETag support, compression, SPA fallback |
| WebSocket ping/pong | Manual implementation | axum's built-in | Automatically handled at protocol level |

**Key insight:** The Rust async ecosystem has mature solutions for WebSocket relay patterns. The axum chat example is essentially 80% of what the relay needs.

## Common Pitfalls

### Pitfall 1: Forgetting to Remove Disconnected Clients

**What goes wrong:** When a browser or mac-client disconnects, their sender remains in the state map. Broadcasting to a closed sender either panics or silently fails, leaving stale entries that consume memory.

**Why it happens:** The WebSocket recv loop exits on disconnect, but cleanup code is forgotten or errors prevent it from running.

**How to avoid:**
1. Use `Drop` trait or explicit cleanup in a `finally`-style block
2. Always wrap WebSocket handling in a scope that cleans up on any exit path
3. Periodically sweep for dead connections (heartbeat)

**Warning signs:** Memory usage grows over time, error logs show "send to closed channel"

### Pitfall 2: Session Code Collision

**What goes wrong:** Two mac-clients get the same session code, causing message routing confusion.

**Why it happens:** Short codes (like 6 characters) have collision probability, especially under load.

**How to avoid:**
1. Check if code exists before assigning: `while sessions.contains_key(&code) { code = generate_new(); }`
2. Use longer codes (8+ chars) for production
3. Include timestamp component for additional entropy

**Warning signs:** Browser connects but sees wrong terminal session

### Pitfall 3: Binary vs Text Message Confusion

**What goes wrong:** Terminal data (binary) is sent as Text message, causing UTF-8 validation errors. Or control messages (JSON) sent as Binary, requiring extra decode step.

**Why it happens:** axum's Message enum has both Binary and Text variants, easy to use wrong one.

**How to avoid:**
1. Establish clear convention: Binary for terminal I/O, Text for JSON control
2. Validate message type on receive before processing
3. Log mismatched message types as warnings

**Warning signs:** "invalid UTF-8" errors, garbled terminal output, JSON parse failures

### Pitfall 4: No Backpressure on Slow Browsers

**What goes wrong:** Fast terminal output (e.g., `cat large_file.txt`) overwhelms slow browser connection, causing unbounded memory growth.

**Why it happens:** Broadcast channels buffer messages when receivers are slow.

**How to avoid:**
1. Use bounded channels with appropriate capacity (100-1000 messages)
2. Drop oldest messages when buffer full (acceptable for terminal output)
3. Consider flow control protocol for critical applications

**Warning signs:** Memory spikes during high output, browser freezes then catches up

### Pitfall 5: WebSocket Connection Not Closed Properly

**What goes wrong:** Server or client doesn't send Close frame, leaving connections in limbo state.

**Why it happens:** Abrupt process termination, network issues, or forgetting to handle close.

**How to avoid:**
1. Always send Close frame before dropping connection
2. Handle incoming Close frame and respond with Close
3. Set reasonable read/write timeouts

**Warning signs:** "connection reset" errors, lingering TCP connections

## Code Examples

Verified patterns from official sources:

### WebSocket Router Setup
```rust
// Source: axum examples
use axum::{Router, routing::get};
use tower_http::cors::CorsLayer;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .nest_service("/", ServeEmbed::<Assets>::new())
        .layer(CorsLayer::permissive())  // Configure properly for production
        .with_state(state)
}
```

### Session Code Generation
```rust
// Source: https://docs.rs/nanoid/latest/nanoid/
use nanoid::nanoid;

fn generate_session_code() -> String {
    // 6 characters, uppercase alphanumeric for easy typing
    let alphabet: [char; 36] = [
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K',
        'L', 'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'U', 'V',
        'W', 'X', 'Y', 'Z', '2', '3', '4', '5', '6', '7',
        '8', '9', '0', '1', 'I', 'O',  // Included for full set
    ];
    // Exclude confusing characters (0/O, 1/I/l) for production
    nanoid!(6, &alphabet)
}
```

### Static Asset Embedding with SPA Fallback
```rust
// Source: https://docs.rs/axum-embed/latest/axum_embed/
use rust_embed::RustEmbed;
use axum_embed::ServeEmbed;

#[derive(RustEmbed, Clone)]
#[folder = "web-ui/dist"]
struct Assets;

// In router setup:
let serve_assets = ServeEmbed::<Assets>::new();
let app = Router::new()
    .route("/ws", get(ws_handler))
    .nest_service("/", serve_assets);  // Fallback serves index.html for SPA
```

### Session Expiration with DelayQueue
```rust
// Source: https://docs.rs/tokio-util/latest/tokio_util/time/delay_queue/struct.DelayQueue.html
use tokio_util::time::delay_queue::{DelayQueue, Key};
use std::time::Duration;

struct SessionManager {
    sessions: DashMap<String, SessionData>,
    expiration_queue: Mutex<DelayQueue<String>>,  // Session codes
    expiration_keys: DashMap<String, Key>,         // Code -> Key for reset
}

impl SessionManager {
    async fn register_session(&self, code: String, data: SessionData) {
        self.sessions.insert(code.clone(), data);

        // Set 1-hour expiration
        let mut queue = self.expiration_queue.lock().await;
        let key = queue.insert(code.clone(), Duration::from_secs(3600));
        self.expiration_keys.insert(code, key);
    }

    async fn refresh_session(&self, code: &str) {
        if let Some(key) = self.expiration_keys.get(code) {
            let mut queue = self.expiration_queue.lock().await;
            queue.reset(&key, Duration::from_secs(3600));
        }
    }

    // Run in background task
    async fn cleanup_expired(&self) {
        loop {
            let expired = {
                let mut queue = self.expiration_queue.lock().await;
                queue.poll_expired().await
            };
            if let Some(expired) = expired {
                let code = expired.into_inner();
                self.sessions.remove(&code);
                self.expiration_keys.remove(&code);
            }
        }
    }
}
```

### Protocol Message Types
```rust
// Source: Project research - ARCHITECTURE.md
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlMessage {
    // Mac-client -> Relay
    Register { client_id: String },
    SessionList { sessions: Vec<SessionInfo> },

    // Relay -> Mac-client
    Registered { code: String },
    BrowserConnected { browser_id: String },
    BrowserDisconnected { browser_id: String },

    // Browser -> Relay
    Auth { session_code: String },
    SelectSession { session_id: String },

    // Relay -> Browser
    AuthSuccess { sessions: Vec<SessionInfo> },
    AuthFailed { reason: String },
    SessionSelected { session_id: String },

    // Bidirectional
    Resize { cols: u16, rows: u16 },
    Error { message: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,  // e.g., "zsh"
    pub connected_at: String,
}

// Binary messages are raw terminal I/O, not JSON
// Text messages are JSON-encoded ControlMessage
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| actix-web for WebSocket | axum 0.8 | 2024-2025 | Better DX, same performance |
| Arc<Mutex<HashMap>> | DashMap | N/A | Less contention, cleaner API |
| Manual timer tasks | DelayQueue | N/A | Correct concurrent expiration handling |
| tower-http ServeDir | rust-embed + axum-embed | N/A | Single binary deployment |

**Deprecated/outdated:**
- `axum-extra::ws` - Merged into main `axum` crate as `axum::extract::ws`
- Old path syntax `/:param` - Now `/{param}` in axum 0.8

## Open Questions

Things that couldn't be fully resolved:

1. **Optimal broadcast channel capacity**
   - What we know: 100 is the chat example default, works for chat
   - What's unclear: Optimal for terminal data at high throughput (1000+ chars/sec)
   - Recommendation: Start with 1000, monitor dropped messages, tune based on real usage

2. **Session code expiration duration**
   - What we know: Need some timeout to prevent abandoned sessions
   - What's unclear: What's reasonable - 1 hour? 24 hours? Indefinite with activity refresh?
   - Recommendation: 1 hour with activity-based refresh, make configurable via env var

3. **Multiple browsers viewing same session**
   - What we know: Architecture supports it (broadcast channel)
   - What's unclear: Should all browsers be able to send input, or only one "controller"?
   - Recommendation: Start with all can send (collaborative), add "view-only" mode later if needed

## Sources

### Primary (HIGH confidence)
- [axum WebSocket example](https://github.com/tokio-rs/axum/blob/main/examples/websockets/src/main.rs) - Handler patterns
- [axum chat example](https://github.com/tokio-rs/axum/blob/main/examples/chat/src/main.rs) - Broadcast channel pattern
- [axum::extract::ws docs](https://docs.rs/axum/latest/axum/extract/ws/index.html) - WebSocket API reference
- [rust-embed docs](https://docs.rs/rust-embed/latest/rust_embed/) - Embedding configuration
- [axum-embed docs](https://docs.rs/axum-embed/latest/axum_embed/) - ServeEmbed usage
- [DashMap docs](https://docs.rs/dashmap/latest/dashmap/struct.DashMap.html) - Concurrent HashMap
- [tokio-util DelayQueue](https://docs.rs/tokio-util/latest/tokio_util/time/delay_queue/struct.DelayQueue.html) - TTL management
- [nanoid docs](https://docs.rs/nanoid/latest/nanoid/) - ID generation

### Secondary (MEDIUM confidence)
- [rust-websocket-relay](https://github.com/AlexSua/rust-websocket-relay) - Relay architecture patterns
- [Axum WebSocket with state management](https://medium.com/@mikecode/axum-websocket-468736a5e1c7) - Real-world patterns
- [tokio shared state tutorial](https://tokio.rs/tokio/tutorial/shared-state) - State management guidance

### Tertiary (LOW confidence)
- [WebSocket backpressure article](https://skylinecodes.substack.com/p/backpressure-in-websocket-streams) - Flow control concepts

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All crates verified in existing research, versions confirmed
- Architecture: HIGH - axum examples provide exact patterns needed
- Pitfalls: MEDIUM - Based on community reports and common patterns, not direct experience

**Research date:** 2026-02-06
**Valid until:** 60 days (stack is stable, axum 0.8 is recent but established)
