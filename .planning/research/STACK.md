# Stack Research: Rust Terminal Remote

**Project:** Terminal Remote v2.0 - Rust Rewrite
**Researched:** 2026-02-06
**Research Method:** Web search + official documentation verification
**Overall Confidence:** HIGH (versions verified via crates.io/lib.rs)

---

## Recommended Stack

### Mac Menu Bar App

| Crate | Version | Purpose | Confidence |
|-------|---------|---------|------------|
| tray-icon | ^0.21 | System tray icon with menu | HIGH |
| muda | ^0.17 | Cross-platform menu utilities | HIGH |
| image | ^0.25 | Icon loading/conversion | HIGH |

**Why tray-icon + muda:**
- Maintained by tauri-apps team, battle-tested in Tauri apps
- Works standalone without full Tauri framework overhead
- Native macOS integration (no webview required)
- Active development: tray-icon 0.21.3 released Jan 2026, muda 0.17.1 released Jul 2025
- Provides TrayIconBuilder API with menu attachment and event handling
- ~970K monthly downloads for muda indicates strong adoption

**macOS-specific requirements:**
- Event loop and tray icon must be created on main thread
- Use `TrayIconBuilder::new().with_menu(menu).with_tooltip(text).build()`
- Events via channel receivers compatible with winit/tao event loops

**Example pattern:**
```rust
use tray_icon::{TrayIconBuilder, menu::Menu};
use muda::{Menu, MenuItem, PredefinedMenuItem};

let menu = Menu::new();
menu.append(&MenuItem::new("Status: ABC123", true, None))?;
menu.append(&PredefinedMenuItem::separator())?;
menu.append(&MenuItem::new("Quit", true, None))?;

let tray = TrayIconBuilder::new()
    .with_menu(Box::new(menu))
    .with_tooltip("Terminal Remote")
    .with_icon(icon)
    .build()?;
```

**Why NOT cacao:**
- Version 0.4.0-beta2 (Aug 2023) - stale, beta quality
- More powerful but higher complexity for simple menu bar app
- tray-icon/muda provide exactly what we need

**Why NOT full Tauri:**
- Overkill for menu bar app with no window UI
- Adds webview runtime (~2-3MB) we don't need
- tray-icon/muda are extracted from Tauri, give us just the tray functionality

### WebSocket Server

| Crate | Version | Purpose | Confidence |
|-------|---------|---------|------------|
| axum | ^0.8 | HTTP/WebSocket framework | HIGH |
| tokio | ^1.43 | Async runtime | HIGH |
| tokio-tungstenite | ^0.26 | WebSocket protocol (via axum) | HIGH |
| tower-http | ^0.6 | HTTP middleware (CORS, compression) | HIGH |

**Why axum:**
- From Tokio team, first-class async/tower integration
- Built-in WebSocket support via `axum::extract::WebSocket`
- Better developer experience than actix-web (2025 consensus)
- Version 0.8.0 released Jan 2025 with improved ergonomics
- New path syntax `/{param}` aligns with OpenAPI
- Nearly identical performance to actix-web with lower memory usage

**axum 0.8 key changes:**
- Path syntax: `/{single}` and `/{*many}` (was `/:single`, `/*many`)
- Improved `Option<T>` extractor with `OptionalFromRequestParts`
- Removed `#[async_trait]` requirement (Rust 1.75+ native async traits)

**WebSocket pattern with axum:**
```rust
use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    routing::get,
    Router,
};

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        // Binary messages for terminal I/O
        // Text messages for control protocol
    }
}

let app = Router::new().route("/ws", get(ws_handler));
```

**Why NOT actix-web:**
- Steeper learning curve (actor model)
- Less idiomatic Rust patterns
- Performance difference negligible for this use case
- axum ecosystem momentum is stronger in 2025-2026

**Why NOT raw tokio-tungstenite:**
- Need HTTP server anyway for static assets
- axum provides WebSocket + HTTP in unified API
- Would duplicate routing/middleware logic

### Static Asset Embedding

| Crate | Version | Purpose | Confidence |
|-------|---------|---------|------------|
| rust-embed | ^8.11 | Embed files at compile time | HIGH |
| axum-embed | ^0.2 | Serve rust-embed with axum | HIGH |

**Why rust-embed:**
- Version 8.11.0 released Jan 2026, actively maintained
- 51 stable releases across 7 major versions
- Features: compression, debug-embed, include-exclude glob patterns
- In debug mode, reads from filesystem (hot reload)
- In release mode, embeds in binary
- Direct axum integration via axum-embed crate

**Embedding pattern:**
```rust
use rust_embed::Embed;
use axum_embed::ServeEmbed;

#[derive(Embed, Clone)]
#[folder = "web-ui/dist"]
struct Assets;

let app = Router::new()
    .route("/ws", get(ws_handler))
    .nest_service("/", ServeEmbed::<Assets>::new());
```

**Key features to enable:**
```toml
[dependencies]
rust-embed = { version = "8.11", features = ["compression"] }
axum-embed = "0.2"
```

**Why NOT include_dir:**
- More manual integration required
- rust-embed's axum integration is turnkey
- Compression support built-in

**Why NOT tower-http ServeDir:**
- Serves from filesystem, not embedded
- Good for dev, but we want single binary for deployment
- rust-embed gives us both (debug vs release behavior)

### Shell Integration IPC

| Approach | Technology | Confidence |
|----------|------------|------------|
| Primary | Unix domain socket (std + tokio) | HIGH |
| Fallback | interprocess crate | MEDIUM |

**Why Unix domain sockets:**
- Native to macOS, no external dependencies
- Bidirectional communication
- Higher throughput than named pipes
- Standard library support: `std::os::unix::net::UnixListener`
- Tokio support: `tokio::net::UnixListener`
- Socket path: `~/.terminal-remote/socket` or `/tmp/terminal-remote-{uid}.sock`

**Shell integration approach:**
```bash
# In .zshrc
if [[ -S "$HOME/.terminal-remote/socket" ]]; then
  # Connect this shell session to mac-client
  exec > >(nc -U "$HOME/.terminal-remote/socket") 2>&1
  # Or use zsh's builtin: zmodload zsh/net/socket
fi
```

**Why NOT interprocess crate:**
- Adds dependency for something stdlib does well on macOS
- interprocess (v2.3.0) is good for cross-platform, but we're macOS-only
- Direct tokio::net::UnixListener is simpler

**Why NOT TCP localhost:**
- Port conflicts possible
- Unix sockets are faster (no TCP overhead)
- Better security (filesystem permissions)

**Why NOT named pipes (FIFO):**
- Unidirectional, would need two pipes
- Unix sockets are bidirectional by design
- More complex shell integration

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Menu bar | tray-icon + muda | cacao | Beta quality, overcomplicated for tray app |
| Menu bar | tray-icon + muda | Full Tauri | Overkill, adds webview we don't need |
| Menu bar | tray-icon + muda | tray-item | Less maintained, simpler API but fewer features |
| WebSocket | axum | actix-web | Steeper learning curve, actor model complexity |
| WebSocket | axum | warp | Less active development, axum is successor |
| WebSocket | axum | raw tungstenite | Need HTTP anyway, axum unifies both |
| Embedding | rust-embed | include_dir | No axum integration, no compression |
| Embedding | rust-embed | static files | Want single binary, not file dependencies |
| IPC | Unix socket | TCP localhost | Port conflicts, slower, less secure |
| IPC | Unix socket | Named pipe | Unidirectional, need two pipes |
| IPC | Unix socket | interprocess | Over-abstraction for macOS-only target |

---

## Integration Notes

### How Pieces Fit Together

```
                    +------------------+
                    |   relay-server   |
                    |   (Rust binary)  |
                    +------------------+
                    | axum HTTP server |
                    | - /ws WebSocket  |
                    | - /* static UI   |
                    | rust-embed assets|
                    +--------+---------+
                             |
              WebSocket      |      WebSocket
           +--------+--------+--------+--------+
           |                                   |
           v                                   v
  +------------------+                +------------------+
  |   mac-client     |                |     Browser      |
  |  (Rust binary)   |                |    (xterm.js)    |
  +------------------+                +------------------+
  | tray-icon + muda |
  | Unix socket IPC  |
  +--------+---------+
           |
   Unix domain socket
           |
           v
  +------------------+
  |  Shell session   |
  |  (zsh + hook)    |
  +------------------+
```

### Dependency Graph

```toml
# mac-client/Cargo.toml
[dependencies]
tray-icon = "0.21"
muda = "0.17"
image = "0.25"
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = "0.26"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# relay-server/Cargo.toml
[dependencies]
axum = { version = "0.8", features = ["ws"] }
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.6", features = ["cors"] }
rust-embed = { version = "8.11", features = ["compression"] }
axum-embed = "0.2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
nanoid = "0.4"
```

### Message Protocol

Keep same binary/text split from v1.0:
- Binary messages: Raw terminal I/O (no parsing overhead)
- Text messages: JSON control messages (resize, auth, etc.)

```rust
// Shared types (could be separate crate)
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum ControlMessage {
    Auth { session_code: String },
    Resize { cols: u16, rows: u16 },
    SessionList { sessions: Vec<SessionInfo> },
}
```

### Event Loop Integration (mac-client)

tray-icon requires running on main thread with event loop:

```rust
use tray_icon::TrayIconEvent;
use muda::MenuEvent;

fn main() {
    // Initialize tray on main thread
    let tray = build_tray();

    // Spawn tokio runtime for async work
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Main event loop
    loop {
        // Handle tray events
        if let Ok(event) = TrayIconEvent::receiver().try_recv() {
            handle_tray_event(event);
        }
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            handle_menu_event(event, &rt);
        }

        // Small sleep to not busy-wait
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
```

---

## Version Verification

All versions verified 2026-02-06 via lib.rs/crates.io:

| Crate | Verified Version | Release Date |
|-------|-----------------|--------------|
| tray-icon | 0.21.3 | Jan 3, 2026 |
| muda | 0.17.1 | Jul 29, 2025 |
| axum | 0.8.x | Jan 1, 2025 |
| tokio-tungstenite | 0.26.2+ | 2025 |
| rust-embed | 8.11.0 | Jan 14, 2026 |
| interprocess | 2.3.0 | Feb 4, 2026 |

---

## Sources

- [tray-icon GitHub](https://github.com/tauri-apps/tray-icon) - Tauri team's tray icon library
- [muda lib.rs](https://lib.rs/crates/muda) - Menu utilities documentation
- [axum 0.8.0 announcement](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0) - Official Tokio blog
- [rust-embed lib.rs](https://lib.rs/crates/rust-embed) - Embedding documentation
- [axum-embed crates.io](https://crates.io/crates/axum-embed) - axum integration for rust-embed
- [interprocess lib.rs](https://lib.rs/crates/interprocess) - IPC library documentation
- [Axum vs Actix-Web 2025](https://medium.com/@indrajit7448/axum-vs-actix-web-the-2025-rust-web-framework-war-performance-vs-dx-17d0ccadd75e) - Framework comparison

---

## Roadmap Implications

1. **Phase 1 - Relay Server:** Start with axum + rust-embed. Well-documented, straightforward.
   - WebSocket handler
   - Static asset serving
   - Session code generation

2. **Phase 2 - Mac Client:** tray-icon + muda for menu bar, then add WebSocket client.
   - Menu bar with session code display
   - WebSocket connection to relay
   - Status updates in menu

3. **Phase 3 - Shell Integration:** Unix socket IPC, then .zshrc hook.
   - Socket listener in mac-client
   - Shell hook script
   - PTY handling for terminal I/O

4. **Phase 4 - Integration:** Connect all pieces, end-to-end testing.
   - Shell -> mac-client -> relay -> browser flow
   - Reconnection handling
   - Multiple session support

**Research flags:**
- PTY handling in Rust (may need deeper research for Phase 3)
- Shell integration edge cases (different zsh configurations)
- macOS code signing for distribution
