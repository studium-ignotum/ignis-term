# Architecture Research: Rust Terminal Remote

**Domain:** Universal terminal remote control with shell integration
**Researched:** 2026-02-06
**Confidence:** MEDIUM-HIGH (Rust ecosystem verified, shell integration pattern needs prototyping)

## Component Overview

```
+------------------+                    +-------------------+
|  Shell Session   |                    |     Browser       |
|  (any terminal)  |                    |   (xterm.js UI)   |
+--------+---------+                    +---------+---------+
         |                                        |
         | Unix Domain Socket                     | WebSocket (WSS)
         | (local IPC)                            |
         v                                        v
+--------+---------+                    +---------+---------+
|    Mac Client    |                    |   Relay Server    |
| (menu bar app)   +<------------------>+  (Rust + Web UI)  |
|   Rust binary    |    WebSocket       |   Rust binary     |
+------------------+                    +-------------------+
```

**Three binaries:**
1. **terminal-remote-attach** - Shell integration helper (Rust), lives in PATH
2. **mac-client** - Menu bar app (Rust), manages sessions and relay connection
3. **relay-server** - WebSocket server with embedded web UI (Rust)

## Shell Integration Design

### The Core Challenge

Universal terminal support requires capturing I/O from shells running in ANY terminal emulator (iTerm2, Terminal.app, VS Code, Zed, etc.). Unlike the v1.0 iTerm2-specific coprocess approach, we cannot rely on terminal-specific APIs.

### Recommended Approach: PTY Interposition

The shell integration script creates a PTY layer between the terminal emulator and the user's shell, enabling I/O capture without terminal-specific APIs.

```
+------------------+     +-----------------------+     +----------------+
| Terminal App     |     | terminal-remote-attach|     | User's Shell   |
| (iTerm2, etc.)   |<--->| (PTY master owner)    |<--->| (zsh on PTY    |
| owns outer PTY   |     | relays + copies       |     |  slave)        |
+------------------+     +----------+------------+     +----------------+
                                    |
                                    | Unix Socket
                                    v
                         +----------+------------+
                         |      Mac Client       |
                         +----------+------------+
                                    |
                                    | WebSocket
                                    v
                         +----------+------------+
                         |    Relay Server       |
                         +----------+------------+
```

### IPC Mechanism: Unix Domain Socket

**Why Unix sockets over alternatives:**

| Mechanism | Pros | Cons | Decision |
|-----------|------|------|----------|
| Unix Domain Socket | Fast, bidirectional, file-based discovery | Unix-only | **Recommended** |
| Named Pipe (FIFO) | Simple | Unidirectional, need two | No |
| TCP localhost | Cross-platform | Port conflicts, firewall | No |
| Shared Memory | Fastest | Complex synchronization | Overkill |

**Socket location:** `/tmp/terminal-remote.sock` (or `$XDG_RUNTIME_DIR/terminal-remote.sock`)

**Protocol:** Length-prefixed JSON messages over the socket.

```rust
// Message framing: 4-byte little-endian length prefix + JSON payload
struct Message {
    msg_type: MessageType,
    session_id: String,
    payload: serde_json::Value,
}

enum MessageType {
    Register,       // New session connecting
    Unregister,     // Session disconnecting
    TerminalData,   // PTY output (shell -> mac-client)
    TerminalInput,  // Keyboard input (mac-client -> shell)
    Resize,         // Terminal size change
    Heartbeat,      // Keep-alive
}
```

### Session Lifecycle

```
1. Shell starts (.zshrc executed)
   |
   v
2. Check for mac-client socket
   |-- Socket missing --> Normal shell (no integration)
   |-- Socket exists -->
   |
   v
3. terminal-remote-attach connects to mac-client
   |
   v
4. Create PTY pair (master + slave)
   |
   v
5. Fork: child execs user's shell on PTY slave
   |
   v
6. Parent (attach binary):
   - Registers session with mac-client (sends session_id, tty, shell)
   - Enters relay loop:
     - stdin -> PTY master (user typing)
     - PTY master -> stdout (terminal display)
     - PTY master -> mac-client socket (I/O copy)
     - mac-client socket -> PTY master (remote input)
   |
   v
7. On shell exit: send Unregister, cleanup, exit
```

### Shell Integration Script

**User's .zshrc addition (one line):**

```zsh
# Terminal Remote integration
[[ -S /tmp/terminal-remote.sock ]] && exec terminal-remote-attach
```

**How it works:**
1. `-S` tests if the socket file exists (mac-client is running)
2. `exec` replaces the current shell with `terminal-remote-attach`
3. `terminal-remote-attach` then runs the user's real shell as a child

**Fallback behavior:** When mac-client isn't running, the socket doesn't exist, and the shell starts normally. Zero overhead.

### terminal-remote-attach Binary Design

```rust
// Rust implementation using portable-pty or nix crate

use nix::pty::{openpty, OpenptyResult};
use nix::unistd::{fork, ForkResult, execvp};
use tokio::net::UnixStream;

async fn main() -> Result<()> {
    // 1. Connect to mac-client
    let socket = UnixStream::connect("/tmp/terminal-remote.sock").await?;

    // 2. Create PTY
    let OpenptyResult { master, slave } = openpty(None, None)?;

    // 3. Fork and exec shell
    match unsafe { fork()? } {
        ForkResult::Child => {
            // Set up slave as controlling terminal
            // exec user's shell
        }
        ForkResult::Parent { child } => {
            // Generate session ID
            let session_id = uuid::Uuid::new_v4().to_string();

            // Register with mac-client
            send_message(&socket, Message::Register {
                session_id: session_id.clone(),
                shell: std::env::var("SHELL")?,
                pid: child.as_raw(),
            }).await?;

            // Enter relay loop
            relay_loop(master, socket, session_id).await?;
        }
    }
}

async fn relay_loop(
    pty_master: OwnedFd,
    socket: UnixStream,
    session_id: String,
) -> Result<()> {
    // Use tokio for async I/O multiplexing:
    // - Read from stdin -> write to PTY master
    // - Read from PTY master -> write to stdout AND socket
    // - Read from socket -> write to PTY master
}
```

**Key crates:**
- `nix` or `portable-pty` for PTY operations
- `tokio` for async I/O
- `serde_json` for message serialization

## Mac Client Design

### Menu Bar Integration

**Recommended approach:** [tray-item](https://lib.rs/crates/tray-item) crate for cross-platform menu bar support, or [tauri](https://tauri.app/) for a more full-featured solution.

For a minimal menu bar app without a full UI framework:

```rust
use tray_item::TrayItem;

fn main() {
    let mut tray = TrayItem::new("Terminal Remote", "icon").unwrap();

    // Display session code
    tray.add_label(&format!("Code: {}", session_code)).unwrap();

    // Session list submenu
    tray.add_menu_item("Sessions", || {
        // Show connected sessions
    }).unwrap();

    // Quit option
    tray.add_menu_item("Quit", || {
        std::process::exit(0);
    }).unwrap();

    // Run event loop
    loop {
        std::thread::park();
    }
}
```

**For Tauri-based approach (recommended for richer UI):**
- Use Tauri's system tray API
- Can show session list in a popup window
- Supports native macOS notifications

### Session Management

```rust
struct SessionManager {
    sessions: HashMap<String, Session>,
    socket_listener: UnixListener,
    relay_connection: Option<WebSocket>,
}

struct Session {
    id: String,
    shell: String,
    pid: u32,
    connected_at: Instant,
    stream: UnixStream,  // Connection to terminal-remote-attach
}

impl SessionManager {
    async fn accept_session(&mut self) -> Result<()> {
        let (stream, _) = self.socket_listener.accept().await?;

        // Read registration message
        let msg: Message = read_message(&stream).await?;

        if let MessageType::Register { session_id, shell, pid } = msg.msg_type {
            let session = Session {
                id: session_id.clone(),
                shell,
                pid,
                connected_at: Instant::now(),
                stream,
            };

            self.sessions.insert(session_id.clone(), session);

            // Notify relay of new session
            if let Some(relay) = &self.relay_connection {
                relay.send(RelayMessage::SessionConnected {
                    session_id,
                    name: shell,
                }).await?;
            }
        }
        Ok(())
    }

    async fn route_terminal_data(&mut self, session_id: &str, data: &[u8]) {
        // Forward to relay for browser display
        if let Some(relay) = &self.relay_connection {
            relay.send(RelayMessage::TerminalData {
                session_id: session_id.to_string(),
                data: data.to_vec(),
            }).await.ok();
        }
    }

    async fn route_remote_input(&mut self, session_id: &str, data: &[u8]) {
        // Forward from browser to specific session
        if let Some(session) = self.sessions.get_mut(session_id) {
            send_message(&session.stream, Message::TerminalInput {
                session_id: session_id.to_string(),
                data: data.to_vec(),
            }).await.ok();
        }
    }
}
```

### Relay Communication

```rust
use tokio_tungstenite::{connect_async, WebSocketStream};

struct RelayClient {
    ws: WebSocketStream<...>,
    session_code: String,
}

impl RelayClient {
    async fn connect(relay_url: &str) -> Result<Self> {
        let (ws, _) = connect_async(relay_url).await?;

        // Register with relay, receive session code
        ws.send(Message::text(json!({
            "type": "register",
            "client_id": machine_id(),
        }).to_string())).await?;

        let response = ws.next().await?;
        let session_code = parse_session_code(&response)?;

        Ok(Self { ws, session_code })
    }

    async fn run(&mut self, session_manager: Arc<Mutex<SessionManager>>) {
        loop {
            select! {
                // Receive from relay (browser input)
                Some(msg) = self.ws.next() => {
                    self.handle_relay_message(msg, &session_manager).await;
                }
                // Send terminal data to relay
                Some(data) = session_manager.lock().await.next_data() => {
                    self.ws.send(data).await.ok();
                }
            }
        }
    }
}
```

## Relay Server Design

### Technology Choice: Axum

**Why Axum:**

| Framework | WebSocket | Static Files | Embedded Assets | Performance | Decision |
|-----------|-----------|--------------|-----------------|-------------|----------|
| Axum | Native (tokio) | tower-http | rust-embed | Excellent | **Recommended** |
| Actix-web | Good | Good | Good | Excellent | Alternative |
| Warp | Good | Good | Limited | Good | No |

**Key crates:**
- `axum` - Web framework
- `axum-extra` - WebSocket support
- `rust-embed` or `memory-serve` - Embed static assets at compile time
- `tokio` - Async runtime
- `tower-http` - Middleware (compression, CORS)

### WebSocket Handling

```rust
use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    routing::get,
    Router,
};

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // Determine client type from first message
    let first_msg = receiver.next().await;

    match parse_client_type(&first_msg) {
        ClientType::MacClient => {
            handle_mac_client(sender, receiver, state).await;
        }
        ClientType::Browser { session_code } => {
            handle_browser(sender, receiver, session_code, state).await;
        }
    }
}

async fn handle_mac_client(
    sender: SplitSink<...>,
    receiver: SplitStream<...>,
    state: AppState,
) {
    // Generate session code
    let code = generate_session_code();

    // Store client connection
    state.clients.lock().await.insert(code.clone(), ClientConnection {
        sender,
        sessions: HashMap::new(),
    });

    // Send code to client
    sender.send(Message::text(json!({
        "type": "registered",
        "code": code,
    }).to_string())).await.ok();

    // Process incoming messages
    while let Some(msg) = receiver.next().await {
        // Route terminal data to subscribed browsers
    }
}

async fn handle_browser(
    sender: SplitSink<...>,
    receiver: SplitStream<...>,
    session_code: String,
    state: AppState,
) {
    // Find mac-client with this code
    let client = state.clients.lock().await.get(&session_code);

    if client.is_none() {
        sender.send(Message::text(json!({
            "type": "error",
            "message": "Invalid session code",
        }).to_string())).await.ok();
        return;
    }

    // Subscribe browser to session updates
    // Process browser input, forward to mac-client
}
```

### Static Asset Serving with rust-embed

```rust
use axum::{
    body::Body,
    http::{header, StatusCode},
    response::Response,
    routing::get,
    Router,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "ui/dist"]  // SvelteKit build output
struct Assets;

async fn static_handler(uri: axum::http::Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream();

            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(content.data.to_vec()))
                .unwrap()
        }
        None => {
            // SPA fallback: serve index.html for client-side routing
            if let Some(content) = Assets::get("index.html") {
                Response::builder()
                    .header(header::CONTENT_TYPE, "text/html")
                    .body(Body::from(content.data.to_vec()))
                    .unwrap()
            } else {
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap()
            }
        }
    }
}

fn app() -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .fallback(static_handler)
}
```

### Session Routing

```rust
struct AppState {
    // Session code -> Mac client connection
    clients: Mutex<HashMap<String, ClientConnection>>,
    // Session code -> Browser connections
    browsers: Mutex<HashMap<String, Vec<BrowserConnection>>>,
}

struct ClientConnection {
    sender: SplitSink<WebSocket, Message>,
    sessions: HashMap<String, SessionInfo>,
}

struct BrowserConnection {
    sender: SplitSink<WebSocket, Message>,
    subscribed_session: Option<String>,
}

impl AppState {
    async fn route_terminal_data(
        &self,
        session_code: &str,
        session_id: &str,
        data: &[u8],
    ) {
        let browsers = self.browsers.lock().await;

        if let Some(browser_list) = browsers.get(session_code) {
            for browser in browser_list {
                // Send to browsers subscribed to this session
                if browser.subscribed_session.as_deref() == Some(session_id) {
                    browser.sender.send(Message::binary(data.to_vec())).await.ok();
                }
            }
        }
    }

    async fn route_browser_input(
        &self,
        session_code: &str,
        session_id: &str,
        data: &[u8],
    ) {
        let clients = self.clients.lock().await;

        if let Some(client) = clients.get(session_code) {
            client.sender.send(Message::text(json!({
                "type": "input",
                "session_id": session_id,
                "data": base64::encode(data),
            }).to_string())).await.ok();
        }
    }
}
```

## Data Flows

### Terminal Output: Shell -> Browser

```
1. User runs command in terminal
   |
   v
2. Shell produces output on PTY slave
   |
   v
3. terminal-remote-attach reads from PTY master
   |
   v
4. Data written to:
   - stdout (displayed in terminal)
   - Unix socket (sent to mac-client)
   |
   v
5. Mac-client receives TerminalData message
   |
   v
6. Mac-client forwards over WebSocket to relay
   |
   v
7. Relay routes to subscribed browser(s)
   |
   v
8. Browser receives WebSocket message
   |
   v
9. xterm.js terminal.write(data)
```

**Latency targets:**
- Local (shell -> mac-client): <1ms
- WAN (mac-client -> browser): 50-200ms depending on network

### User Input: Browser -> Shell

```
1. User types in browser (xterm.js)
   |
   v
2. terminal.onData() callback fires
   |
   v
3. Browser sends WebSocket message to relay
   |
   v
4. Relay routes to mac-client by session code
   |
   v
5. Mac-client receives input message
   |
   v
6. Mac-client sends to terminal-remote-attach via Unix socket
   |
   v
7. terminal-remote-attach writes to PTY master
   |
   v
8. Shell receives input as if typed locally
```

### Session Discovery

```
Browser connects to relay with session code
         |
         v
Relay validates code, finds mac-client
         |
         v
Relay requests session list from mac-client
         |
         v
Mac-client returns list of connected sessions:
[
  { id: "abc123", shell: "zsh", connected: "5m ago" },
  { id: "def456", shell: "zsh", connected: "2m ago" },
]
         |
         v
Relay forwards to browser
         |
         v
Browser displays session picker UI
         |
         v
User selects session
         |
         v
Browser subscribes to session, starts receiving output
```

## Build Order

Based on dependencies and testability, recommended implementation sequence:

### Phase 1: Relay Server Foundation (Week 1)

**Goal:** Deployable relay that serves web UI and handles WebSocket connections.

1. **Axum server skeleton** with WebSocket endpoint
2. **Static asset embedding** (rust-embed) for xterm.js web UI
3. **Session code generation** and client registration
4. **Basic message routing** between clients

**Test:** Browser can connect, see UI, enter session code.

### Phase 2: Mac Client Core (Week 2)

**Goal:** Menu bar app that connects to relay and receives session code.

1. **Menu bar integration** (tray-item or Tauri)
2. **WebSocket client** to relay (tokio-tungstenite)
3. **Session code display** in menu bar
4. **Unix socket listener** for local sessions

**Test:** Mac client shows in menu bar with session code.

### Phase 3: Shell Integration (Week 3)

**Goal:** Sessions auto-connect when mac-client is running.

1. **terminal-remote-attach binary** with PTY handling
2. **Unix socket client** connecting to mac-client
3. **Session registration** protocol
4. **I/O relay loop** (stdin/stdout <-> PTY <-> socket)

**Test:** New terminal sessions appear in mac-client session list.

### Phase 4: End-to-End Data Flow (Week 4)

**Goal:** Browser can view and control terminal sessions.

1. **Terminal data routing** through full pipeline
2. **Browser input routing** back to shell
3. **Session switching** in browser
4. **Terminal resize** propagation

**Test:** Full remote terminal control working.

### Phase 5: Polish and Reliability (Week 5+)

1. **Reconnection handling** for all connections
2. **Heartbeat/keepalive** for connection health
3. **Error recovery** and graceful degradation
4. **Performance optimization** (batching, compression)

## Crate Recommendations

| Component | Crate | Version | Confidence |
|-----------|-------|---------|------------|
| Async runtime | tokio | 1.x | HIGH |
| PTY handling | portable-pty | 0.9.x | HIGH |
| PTY (alternative) | nix | 0.29.x | HIGH |
| Unix sockets | tokio (built-in) | 1.x | HIGH |
| WebSocket client | tokio-tungstenite | 0.24.x | HIGH |
| Web framework | axum | 0.8.x | HIGH |
| Static embedding | rust-embed | 8.x | HIGH |
| Menu bar | tray-item | 0.10.x | MEDIUM |
| Menu bar (alternative) | tauri | 2.x | HIGH |
| Serialization | serde, serde_json | 1.x | HIGH |

## Open Questions

### 1. terminal-remote-attach Distribution

**Question:** How do users install the `terminal-remote-attach` binary?

**Options:**
- Bundled with mac-client installer
- Homebrew formula
- Manual download from releases

**Recommendation:** Mac-client installer copies binary to `/usr/local/bin/`.

### 2. Raw Mode vs Line Mode

**Question:** Does terminal-remote-attach need to put stdin/stdout in raw mode?

**Answer:** Yes. The attach binary must set the terminal to raw mode to pass through all escape sequences, special keys (Ctrl+C, etc.), and not interpret newlines.

```rust
use termios::{tcsetattr, Termios, TCSANOW, ECHO, ICANON, ISIG};

fn set_raw_mode(fd: RawFd) -> Result<Termios> {
    let original = Termios::from_fd(fd)?;
    let mut raw = original.clone();
    raw.c_lflag &= !(ECHO | ICANON | ISIG);
    tcsetattr(fd, TCSANOW, &raw)?;
    Ok(original)  // Return original to restore later
}
```

### 3. Multiple Browser Viewers

**Question:** Can multiple browsers view the same session?

**Answer:** Yes. The relay maintains a list of browser connections per session code. All subscribed browsers receive the same terminal data.

### 4. Clipboard Support

**Question:** How does copy/paste work with remote terminal?

**Answer:**
- Browser copy: Standard browser selection + Cmd/Ctrl+C (xterm.js handles this)
- Browser paste: Cmd/Ctrl+V sends text as terminal input
- OSC 52 clipboard: If shell sends OSC 52 sequence, could forward to browser clipboard API (advanced feature)

## Sources

### Shell Integration & PTY
- [Kitty Shell Integration](https://sw.kovidgoyal.net/kitty/shell-integration/) - OSC escape sequence patterns
- [iTerm2 Shell Integration](https://iterm2.com/shell_integration.html) - Hook implementation reference
- [zsh-async](https://github.com/mafredri/zsh-async) - zpty pseudo-terminal usage
- [Linux terminals, tty, pty and shell](https://dev.to/napicella/linux-terminals-tty-pty-and-shell-192e) - PTY architecture

### Rust Libraries
- [portable-pty](https://lib.rs/crates/portable-pty) - Cross-platform PTY crate
- [nix](https://docs.rs/nix/latest/nix/pty/) - Low-level Unix PTY operations
- [tokio UnixStream](https://docs.rs/tokio/latest/tokio/net/struct.UnixStream.html) - Async Unix sockets
- [tokio-unix-ipc](https://github.com/mitsuhiko/tokio-unix-ipc) - Higher-level IPC wrapper
- [tray-item](https://lib.rs/crates/tray-item) - Simple menu bar API
- [Tauri menubar example](https://github.com/4gray/tauri-menubar-app) - Tauri system tray
- [rust-embed with Axum](https://github.com/pyrossh/rust-embed/blob/master/examples/axum.rs) - Static asset embedding
- [axum-embed](https://docs.rs/axum-embed/latest/axum_embed/) - Axum integration for rust-embed
- [memory-serve](https://docs.rs/memory-serve/latest/memory_serve/) - Alternative asset serving

### Web Framework
- [Axum static file server](https://github.com/tokio-rs/axum/blob/main/examples/static-file-server/src/main.rs) - Official example
- [Axum WebSocket example](https://github.com/tokio-rs/axum/blob/main/examples/websockets/src/main.rs) - WebSocket handling
- [Serving Static Files with Axum](https://benw.is/posts/serving-static-files-with-axum) - Tutorial

---

*Research date: 2026-02-06*
*Confidence: MEDIUM-HIGH - Rust ecosystem well-verified, PTY interposition pattern is established but needs prototyping for this specific use case*
