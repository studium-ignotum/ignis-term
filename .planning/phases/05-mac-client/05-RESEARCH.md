# Phase 5: Mac Client - Research

**Researched:** 2026-02-06
**Domain:** Rust macOS menu bar application with WebSocket client and Unix socket IPC
**Confidence:** HIGH

## Summary

Phase 5 builds the Mac client as a menu bar application that coordinates local shell sessions with the cloud relay server. The Mac client has three main responsibilities: (1) displaying a menu bar icon with status and session information, (2) maintaining a WebSocket connection to the relay server with auto-reconnect, and (3) listening on a Unix domain socket for shell integration connections from Phase 6.

The standard stack is well-established: **tray-icon 0.21.3** and **muda 0.17.1** from the Tauri team provide battle-tested menu bar functionality. **tokio-tungstenite 0.28** handles WebSocket client connections, and **tokio's built-in UnixListener/UnixStream** manages local IPC. The critical architectural constraint is that macOS AppKit requires the tray icon and menu to be created and updated on the main thread, while all async I/O runs on a background Tokio runtime, communicating via channels.

The client will connect to the relay server at startup, receive a 6-character session code, and display it in the menu bar dropdown. When shell integration (Phase 6) connects via Unix socket, terminal data flows through the mac-client to the relay for browser viewing.

**Primary recommendation:** Structure the app with main thread owning the tray icon/menu, background thread running Tokio for WebSocket and Unix socket, and mpsc channels for bidirectional communication between them.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tray-icon | ^0.21 | Menu bar icon with click handling | Tauri-team maintained, 970K+ monthly downloads of muda |
| muda | ^0.17 | Cross-platform menu construction | Re-exported by tray-icon, native macOS menu support |
| tokio | ^1.43 | Async runtime | De facto standard, runs on background thread |
| tokio-tungstenite | ^0.28 | WebSocket client | Full tungstenite protocol, integrates with tokio |
| arboard | ^3.6 | Clipboard operations | 1Password maintained, cross-platform, simple API |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| smappservice-rs | latest | Launch at login (macOS 13+) | CLIENT-11: Start at login option |
| image | ^0.25 | Icon loading and conversion | Converting PNG to RGBA for tray icon |
| serde | ^1 | Serialization | Protocol messages to/from relay |
| serde_json | ^1 | JSON encoding | Control message parsing |
| futures-util | ^0.3 | Stream/Sink extensions | WebSocket split, StreamExt |
| uuid | ^1 | Unique IDs | Client ID generation |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| tray-icon + muda | Full Tauri | Tauri adds webview overhead (2-3MB) we don't need |
| tray-icon + muda | cacao | cacao is beta (0.4.0-beta2), tray-icon is stable |
| tokio-tungstenite | ezsockets | ezsockets has auto-reconnect but adds abstraction |
| arboard | clipboard crate | arboard is more actively maintained, 1Password backing |

**Cargo.toml dependencies:**
```toml
[dependencies]
tray-icon = "0.21"
muda = "0.17"
image = "0.25"
tokio = { version = "1", features = ["full", "sync", "net"] }
tokio-tungstenite = { version = "0.28", features = ["native-tls"] }
futures-util = "0.3"
arboard = "3.6"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4"] }
smappservice-rs = "0.1"  # For login item registration
tracing = "0.1"
tracing-subscriber = "0.3"

[build-dependencies]
# For embedding icon
```

## Architecture Patterns

### Recommended Project Structure
```
mac-client/
├── src/
│   ├── main.rs              # Entry point, main thread event loop
│   ├── app.rs               # Application state, threading coordination
│   ├── tray/
│   │   ├── mod.rs           # Tray icon builder and management
│   │   ├── menu.rs          # Menu construction and updates
│   │   └── icon.rs          # Icon loading, template image handling
│   ├── relay/
│   │   ├── mod.rs           # WebSocket client to relay server
│   │   ├── connection.rs    # Connection state, reconnection logic
│   │   └── messages.rs      # Protocol message types
│   ├── ipc/
│   │   ├── mod.rs           # Unix socket listener
│   │   └── session.rs       # Session tracking for connected shells
│   └── clipboard.rs         # Clipboard operations
├── resources/
│   └── icon.png             # Template icon (black on transparent)
├── Cargo.toml
└── Info.plist               # For app bundle (LSUIElement)
```

### Pattern 1: Main Thread + Background Tokio Runtime
**What:** Run Tokio on a dedicated background thread, main thread owns tray icon
**When to use:** Always for macOS menu bar apps with async I/O

```rust
// Source: https://tokio.rs/tokio/topics/bridging
use std::sync::mpsc;
use std::thread;
use tray_icon::{TrayIconBuilder, TrayIconEvent};
use muda::MenuEvent;
use tokio::runtime::Runtime;

fn main() {
    // Channels for communication
    let (ui_tx, ui_rx) = mpsc::channel::<UiCommand>();  // Background -> UI
    let (bg_tx, bg_rx) = std::sync::mpsc::channel::<BackgroundCommand>();  // UI -> Background

    // Spawn background thread with Tokio runtime
    let bg_handle = thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            run_background_tasks(ui_tx, bg_rx).await;
        });
    });

    // Main thread: create tray icon and run event loop
    let tray = build_tray_icon();

    loop {
        // Handle tray events (clicks, etc.)
        if let Ok(event) = TrayIconEvent::receiver().try_recv() {
            handle_tray_click(event);
        }

        // Handle menu events
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            handle_menu_event(event, &bg_tx);
        }

        // Handle updates from background (session code, status changes)
        if let Ok(cmd) = ui_rx.try_recv() {
            update_tray_menu(cmd, &tray);
        }

        // Small sleep to avoid busy-waiting
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
```

### Pattern 2: WebSocket Client with Auto-Reconnect
**What:** Exponential backoff reconnection when connection drops
**When to use:** CLIENT-02 requires auto-reconnect capability

```rust
// Source: Based on tokio-tungstenite patterns
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio::net::TcpStream;
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;

struct RelayConnection {
    ws: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    relay_url: String,
    session_code: Option<String>,
    reconnect_attempts: u32,
}

impl RelayConnection {
    async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (ws, _) = connect_async(&self.relay_url).await?;
        self.ws = Some(ws);
        self.reconnect_attempts = 0;

        // Register with relay
        self.send_register().await?;
        Ok(())
    }

    async fn run_with_reconnect(&mut self, ui_tx: mpsc::Sender<UiCommand>) {
        loop {
            match self.connect().await {
                Ok(()) => {
                    ui_tx.send(UiCommand::Connected).ok();
                    self.process_messages(&ui_tx).await;
                }
                Err(e) => {
                    tracing::error!("Connection failed: {}", e);
                }
            }

            // Disconnected - attempt reconnect with exponential backoff
            ui_tx.send(UiCommand::Disconnected).ok();

            let delay = Duration::from_secs(
                (2u64).pow(self.reconnect_attempts.min(5))
            );
            self.reconnect_attempts += 1;

            tracing::info!("Reconnecting in {:?}...", delay);
            tokio::time::sleep(delay).await;
        }
    }

    async fn send_register(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ws) = &mut self.ws {
            let msg = serde_json::json!({
                "type": "register",
                "client_id": uuid::Uuid::new_v4().to_string()
            });
            ws.send(tokio_tungstenite::tungstenite::Message::Text(msg.to_string())).await?;
        }
        Ok(())
    }
}
```

### Pattern 3: Unix Socket Listener for Shell IPC
**What:** Accept connections from shell integration (Phase 6)
**When to use:** CLIENT-03 Unix socket listener for shell integration IPC

```rust
// Source: https://docs.rs/tokio/latest/tokio/net/struct.UnixListener.html
use tokio::net::{UnixListener, UnixStream};
use std::collections::HashMap;
use std::path::PathBuf;

struct SessionManager {
    listener: UnixListener,
    sessions: HashMap<String, Session>,
    socket_path: PathBuf,
}

struct Session {
    id: String,
    stream: UnixStream,
    shell: String,
    connected_at: std::time::Instant,
}

impl SessionManager {
    async fn new() -> std::io::Result<Self> {
        let socket_path = PathBuf::from("/tmp/terminal-remote.sock");

        // Remove existing socket file if it exists
        if socket_path.exists() {
            std::fs::remove_file(&socket_path)?;
        }

        let listener = UnixListener::bind(&socket_path)?;

        Ok(Self {
            listener,
            sessions: HashMap::new(),
            socket_path,
        })
    }

    async fn accept_connections(&mut self, ui_tx: mpsc::Sender<UiCommand>) {
        loop {
            match self.listener.accept().await {
                Ok((stream, _addr)) => {
                    self.handle_new_session(stream, &ui_tx).await;
                }
                Err(e) => {
                    tracing::error!("Socket accept error: {}", e);
                }
            }
        }
    }

    async fn handle_new_session(&mut self, stream: UnixStream, ui_tx: &mpsc::Sender<UiCommand>) {
        // Read registration message from shell integration
        // Add to sessions map
        // Notify UI of session count change
        ui_tx.send(UiCommand::SessionCountChanged(self.sessions.len())).ok();
    }
}

impl Drop for SessionManager {
    fn drop(&mut self) {
        // Clean up socket file
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
```

### Pattern 4: Menu Construction with Dynamic Updates
**What:** Build menu with items that update at runtime (session code, status, count)
**When to use:** CLIENT-06, CLIENT-07, CLIENT-09, CLIENT-13

```rust
// Source: https://docs.rs/muda/latest/muda/
use muda::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::TrayIconBuilder;
use std::sync::atomic::{AtomicUsize, Ordering};

struct TrayMenu {
    menu: Menu,
    session_code_item: MenuItem,
    status_item: MenuItem,
    session_count_item: MenuItem,
    copy_code_item: MenuItem,
}

impl TrayMenu {
    fn new() -> Self {
        let menu = Menu::new();

        // Session code display (disabled, just shows text)
        let session_code_item = MenuItem::new("Code: ------", false, None);
        menu.append(&session_code_item).unwrap();

        // Connection status
        let status_item = MenuItem::new("Status: Connecting...", false, None);
        menu.append(&status_item).unwrap();

        // Session count
        let session_count_item = MenuItem::new("Sessions: 0", false, None);
        menu.append(&session_count_item).unwrap();

        menu.append(&PredefinedMenuItem::separator()).unwrap();

        // Copy session code action
        let copy_code_item = MenuItem::with_id("copy_code", "Copy Session Code", true, None);
        menu.append(&copy_code_item).unwrap();

        menu.append(&PredefinedMenuItem::separator()).unwrap();

        // Quit option
        let quit_item = MenuItem::with_id("quit", "Quit", true, None);
        menu.append(&quit_item).unwrap();

        Self {
            menu,
            session_code_item,
            status_item,
            session_count_item,
            copy_code_item,
        }
    }

    fn update_session_code(&self, code: &str) {
        self.session_code_item.set_text(format!("Code: {}", code));
    }

    fn update_status(&self, connected: bool) {
        let status = if connected { "Connected" } else { "Disconnected" };
        self.status_item.set_text(format!("Status: {}", status));
    }

    fn update_session_count(&self, count: usize) {
        self.session_count_item.set_text(format!("Sessions: {}", count));
    }
}
```

### Anti-Patterns to Avoid

- **Calling tray/menu APIs from Tokio tasks:** AppKit requires main thread. Always use channels to send updates to main thread.
- **Blocking the main thread:** Don't do I/O or heavy work on main thread. Use background Tokio runtime.
- **Busy-waiting without sleep:** Main event loop must include small sleep to avoid 100% CPU.
- **Ignoring socket cleanup:** Unix socket files persist after crash. Always remove on startup and shutdown.
- **Unbounded channels:** Use bounded channels to prevent memory growth if one side is slow.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Clipboard copy | NSPasteboard bindings | arboard crate | Cross-platform, handles edge cases, 1Password maintained |
| Launch at login | LaunchAgent plist | smappservice-rs | Apple deprecated old APIs, SMAppService is modern (macOS 13+) |
| Auto-reconnect | Manual timer loops | Exponential backoff pattern | Standard pattern prevents thundering herd on server |
| Menu construction | Raw Cocoa bindings | muda crate | Handles menu item IDs, events, separators correctly |
| Icon loading | Manual PNG parsing | image crate + tray-icon | Handles RGBA conversion, DPI scaling |

**Key insight:** The tray-icon + muda combination handles most macOS menu bar complexity. Don't try to use lower-level Cocoa bindings unless absolutely necessary.

## Common Pitfalls

### Pitfall 1: Main Thread Violation Crashes
**What goes wrong:** Calling tray icon or menu APIs from a Tokio task causes random crashes, EXC_BAD_ACCESS errors, or the icon appearing briefly then disappearing.
**Why it happens:** macOS AppKit requires UI operations on main thread. Tokio tasks run on worker threads.
**How to avoid:**
1. Create tray icon and menu on main thread before starting Tokio runtime
2. Use `std::sync::mpsc` channels (not tokio::sync) to send updates to main thread
3. Main thread polls channel in event loop, applies updates
**Warning signs:** Works intermittently, crashes in release but not debug, "Main Thread Checker" errors

### Pitfall 2: Stale Socket File Blocks Binding
**What goes wrong:** App crashes or fails to start with "Address already in use" after previous crash.
**Why it happens:** Unix socket files persist on filesystem. Crash prevents cleanup code from running.
**How to avoid:**
1. On startup, always try to remove socket file before binding
2. Use Drop trait to clean up on graceful shutdown
3. Consider unique socket name with PID: `/tmp/terminal-remote-{uid}.sock`
**Warning signs:** Second launch fails, works after manual file deletion

### Pitfall 3: WebSocket Reconnect Storm
**What goes wrong:** After server restart, all clients reconnect simultaneously overwhelming the server.
**Why it happens:** Clients reconnect immediately without jitter, creating thundering herd.
**How to avoid:**
1. Use exponential backoff: 1s, 2s, 4s, 8s, 16s, capped at 32s
2. Add random jitter (0-1s) to backoff delay
3. Reset backoff counter only after sustained connection (e.g., 30s)
**Warning signs:** Server CPU spikes on restart, connections fail during recovery

### Pitfall 4: Menu Not Updating
**What goes wrong:** Menu shows stale information after session code changes or sessions connect.
**Why it happens:** muda MenuItem text changes may not be visible until menu is reopened (macOS caching).
**How to avoid:**
1. This is expected macOS behavior - menu updates show on next open
2. For immediate feedback, consider using tooltip which updates instantly
3. Don't recreate entire menu on every change (causes flicker)
**Warning signs:** User sees old session code, confused by outdated info

### Pitfall 5: Template Image Not Working
**What goes wrong:** Menu bar icon appears as solid black square or doesn't adapt to dark mode.
**Why it happens:** Template image must be black-on-transparent PNG, and `icon_as_template(true)` must be set.
**How to avoid:**
1. Create icon as black (#000000) on transparent background
2. Use single channel or grayscale image
3. Call `.icon_as_template(true)` on TrayIconBuilder
4. Test in both light and dark menu bar modes
**Warning signs:** Icon looks wrong in dark mode, icon is solid rectangle

## Code Examples

Verified patterns from official sources:

### Building Tray Icon with Template Image
```rust
// Source: https://docs.rs/tray-icon/latest/tray_icon/
use tray_icon::{TrayIconBuilder, Icon};
use image::io::Reader as ImageReader;

fn build_tray_icon(menu: &TrayMenu) -> tray_icon::TrayIcon {
    // Load icon from embedded bytes
    let icon_bytes = include_bytes!("../resources/icon.png");
    let img = image::load_from_memory(icon_bytes).unwrap();
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let icon = Icon::from_rgba(rgba.into_raw(), width, height).unwrap();

    TrayIconBuilder::new()
        .with_menu(Box::new(menu.menu.clone()))
        .with_tooltip("Terminal Remote")
        .with_icon(icon)
        .with_icon_as_template(true)  // CLIENT-12: Auto dark/light mode
        .build()
        .unwrap()
}
```

### Handling Menu Events
```rust
// Source: https://docs.rs/muda/latest/muda/struct.MenuEvent.html
use muda::MenuEvent;
use arboard::Clipboard;

fn handle_menu_event(
    event: MenuEvent,
    session_code: &Option<String>,
    bg_tx: &std::sync::mpsc::Sender<BackgroundCommand>,
) {
    match event.id.0.as_str() {
        "copy_code" => {
            if let Some(code) = session_code {
                let mut clipboard = Clipboard::new().unwrap();
                clipboard.set_text(code.clone()).unwrap();
                // Could show notification or update menu to confirm
            }
        }
        "quit" => {
            bg_tx.send(BackgroundCommand::Shutdown).ok();
            std::process::exit(0);
        }
        _ => {}
    }
}
```

### Clipboard Copy with Confirmation
```rust
// Source: https://github.com/1Password/arboard
use arboard::Clipboard;

fn copy_to_clipboard(text: &str) -> Result<(), arboard::Error> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text.to_string())?;
    Ok(())
}

// CLIENT-08: Copy session code to clipboard action
fn copy_session_code(code: &str) -> bool {
    match copy_to_clipboard(code) {
        Ok(()) => {
            tracing::info!("Session code copied to clipboard");
            true  // Success - could show visual feedback
        }
        Err(e) => {
            tracing::error!("Failed to copy to clipboard: {}", e);
            false
        }
    }
}
```

### Launch at Login Registration (macOS 13+)
```rust
// Source: https://docs.rs/smappservice-rs/latest/smappservice_rs/
use smappservice_rs::{AppService, ServiceType, ServiceStatus};

fn configure_login_item(enable: bool) -> Result<(), Box<dyn std::error::Error>> {
    let service = AppService::new(ServiceType::MainApp);

    if enable {
        match service.register() {
            Ok(()) => {
                tracing::info!("Registered as login item");
            }
            Err(e) => {
                tracing::error!("Failed to register login item: {}", e);
                // May need user approval in System Settings
                if matches!(service.status(), ServiceStatus::RequiresApproval) {
                    AppService::open_system_settings_login_items()?;
                }
            }
        }
    } else {
        service.unregister()?;
        tracing::info!("Unregistered as login item");
    }

    Ok(())
}
```

### LSUIElement for No Dock Icon
```xml
<!-- Info.plist - Include in app bundle -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>com.example.terminal-remote</string>
    <key>CFBundleName</key>
    <string>Terminal Remote</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>LSUIElement</key>
    <true/>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
</dict>
</plist>
```

Note: When running outside an app bundle during development, the dock icon may still appear. LSUIElement only takes effect when running from a proper `.app` bundle.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| SMLoginItemSetEnabled | SMAppService | macOS 13 (2022) | Old API deprecated, new API required |
| Manual Cocoa bindings | tray-icon + muda | 2023-2024 | Much simpler, cross-platform |
| Thread::park for event loop | Polling with small sleep | N/A | More responsive, handles events properly |
| clipboard crate | arboard | 2022 | arboard more maintained, 1Password backing |

**Deprecated/outdated:**
- **LSUIElement="1"** (string): Use `<true/>` boolean instead
- **LaunchAgents plist**: Use SMAppService for macOS 13+
- **SMJobBless**: Deprecated, use SMAppService
- **NSStatusItem button?.image**: Use tray-icon's Icon type

## Open Questions

Things that couldn't be fully resolved:

1. **Event loop polling interval**
   - What we know: 10ms sleep is common, prevents busy-wait
   - What's unclear: Optimal interval for responsiveness vs CPU usage
   - Recommendation: Start with 10ms, profile and adjust if needed

2. **Multiple simultaneous browser viewers per session**
   - What we know: Architecture supports it (relay broadcasts)
   - What's unclear: Whether mac-client needs to track individual browser connections
   - Recommendation: Mac-client doesn't need to know about browsers, relay handles routing

3. **Session persistence across mac-client restart**
   - What we know: Shell sessions will disconnect when socket closes
   - What's unclear: Whether to support reconnection of existing shells
   - Recommendation: Phase 6 scope - shells should gracefully handle mac-client restart

## Sources

### Primary (HIGH confidence)
- [tray-icon GitHub](https://github.com/tauri-apps/tray-icon) - v0.21.3, main thread requirements
- [tray-icon docs](https://docs.rs/tray-icon/latest/tray_icon/) - TrayIconBuilder API
- [muda docs](https://docs.rs/muda/latest/muda/) - Menu construction, MenuEvent handling
- [tokio bridging guide](https://tokio.rs/tokio/topics/bridging) - Background thread runtime pattern
- [tokio UnixListener](https://docs.rs/tokio/latest/tokio/net/struct.UnixListener.html) - Socket binding, accept
- [tokio-tungstenite](https://docs.rs/tokio-tungstenite/latest/tokio_tungstenite/) - WebSocket client
- [arboard GitHub](https://github.com/1Password/arboard) - v3.6.1, clipboard operations
- [smappservice-rs](https://github.com/gethopp/smappservice-rs) - Login item registration

### Secondary (MEDIUM confidence)
- [Building a Native macOS Menu Bar App in Rust](https://medium.com/@ekfqlwcjswl/building-a-native-macos-menu-bar-app-in-rust-0d55786db083) - Patterns and pitfalls
- [Hopp blog - Rust app start on login](https://www.gethopp.app/blog/rust-app-start-on-login) - SMAppService usage
- [tokio-tungstenite Issue #101](https://github.com/snapview/tokio-tungstenite/issues/101) - Reconnection patterns

### Tertiary (LOW confidence)
- [winit Issue #261](https://github.com/rust-windowing/winit/issues/261) - LSUIElement challenges with Rust frameworks (may need runtime control)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All crates verified, actively maintained, from trusted sources (Tauri, 1Password)
- Architecture: HIGH - Main thread + background Tokio is well-documented pattern
- Pitfalls: HIGH - Based on documented issues in prior research and multiple sources

**Research date:** 2026-02-06
**Valid until:** 60 days (stack is stable, tray-icon 0.21 is recent)
