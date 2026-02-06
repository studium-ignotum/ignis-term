//! Mac Client - Menu Bar Application
//!
//! A macOS menu bar application for managing remote terminal sessions.
//! This module integrates the tray icon, relay client, and PTY proxy manager.
//!
//! On macOS, we must use a proper event loop for the tray icon to appear.
//! We use winit's EventLoop to drive the main thread.

use image::ImageReader;
use mac_client::app::{AppState, BackgroundCommand, UiEvent};
use mac_client::pty::{PtyCommand, PtyEvent, PtyManager};
use mac_client::relay::{RelayClient, RelayCommand, RelayEvent};
use muda::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use smappservice_rs::{AppService, ServiceStatus, ServiceType};
use std::io::{BufRead, BufReader, Cursor};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tray_icon::{TrayIcon, TrayIconBuilder};
use tracing::{debug, error, info, warn};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

// Menu item IDs
const ID_REGEN_CODE: &str = "regen_code";
const ID_COPY_URL: &str = "copy_url";
const ID_COPY_CODE: &str = "copy_code";
const ID_LOGIN_ITEM: &str = "login_item";
const ID_QUIT: &str = "quit";

/// Custom events for our application
#[derive(Debug)]
enum AppEvent {
    TrayIconEvent(tray_icon::TrayIconEvent),
    MenuEvent(muda::MenuEvent),
}

/// Main application state
struct App {
    tray_icon: Option<TrayIcon>,
    app_state: Option<AppState>,
    login_item: Option<CheckMenuItem>,
    bg_tx: Option<mpsc::Sender<BackgroundCommand>>,
    ui_rx: Option<mpsc::Receiver<UiEvent>>,
    bg_handle: Option<thread::JoinHandle<()>>,
    copy_reset_time: Option<Instant>,
    pty_cmd_tx: Option<tokio::sync::mpsc::UnboundedSender<PtyCommand>>,
    cloudflared_pid: Arc<AtomicU32>,
}

impl App {
    fn new() -> Self {
        Self {
            tray_icon: None,
            app_state: None,
            login_item: None,
            bg_tx: None,
            ui_rx: None,
            bg_handle: None,
            copy_reset_time: None,
            pty_cmd_tx: None,
            cloudflared_pid: Arc::new(AtomicU32::new(0)),
        }
    }

    fn handle_menu_event(&mut self, event: muda::MenuEvent) {
        debug!("Menu event: {:?}", event);

        match event.id().0.as_str() {
            ID_REGEN_CODE => {
                info!("Regenerate session code requested");
                if let Some(bg_tx) = &self.bg_tx {
                    let _ = bg_tx.send(BackgroundCommand::ReconnectRelay);
                }
            }
            ID_COPY_URL => {
                if let Some(app_state) = &self.app_state {
                    if let Some(url) = &app_state.tunnel_url {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            if clipboard.set_text(url.clone()).is_ok() {
                                info!("Tunnel URL copied to clipboard: {}", url);
                            }
                        }
                    }
                }
            }
            ID_COPY_CODE => {
                if let Some(app_state) = &self.app_state {
                    if let Some(code) = &app_state.session_code {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            if clipboard.set_text(code.clone()).is_ok() {
                                info!("Session code copied to clipboard: {}", code);
                            }
                        }
                    }
                }
            }
            ID_LOGIN_ITEM => {
                if let Some(login_item) = &self.login_item {
                    let current = login_item.is_checked();
                    let new_state = !current;

                    match configure_login_item(new_state) {
                        Ok(()) => {
                            login_item.set_checked(new_state);
                            info!(
                                "Login item {}: {}",
                                if new_state { "enabled" } else { "disabled" },
                                new_state
                            );
                        }
                        Err(e) => {
                            error!("Failed to configure login item: {}", e);
                            warn!(
                                "Login item registration may require running from a proper .app bundle"
                            );
                        }
                    }
                }
            }
            ID_QUIT => {
                info!("Quit requested, exiting");
                let pid = self.cloudflared_pid.load(Ordering::Relaxed);
                if pid != 0 {
                    info!("Killing cloudflared (pid {})", pid);
                    unsafe { libc::kill(pid as i32, libc::SIGTERM); }
                }
                std::process::exit(0);
            }
            _ => {
                debug!("Unknown menu item clicked: {:?}", event.id());
            }
        }
    }

    fn handle_ui_events(&mut self) {
        if let Some(ui_rx) = &self.ui_rx {
            while let Ok(event) = ui_rx.try_recv() {
                debug!("UI event: {:?}", event);

                if let Some(app_state) = &mut self.app_state {
                    match event {
                        UiEvent::RelayConnected => {
                            info!("Relay connected");
                            app_state.relay_connected = true;
                            app_state.update_status_display();
                        }
                        UiEvent::RelayDisconnected => {
                            info!("Relay disconnected");
                            app_state.relay_connected = false;
                            app_state.session_code = None;
                            app_state.update_status_display();
                            app_state.update_code_display();
                        }
                        UiEvent::SessionCode(code) => {
                            info!("Received session code: {}", code);
                            app_state.session_code = Some(code);
                            app_state.update_code_display();
                        }
                        UiEvent::BrowserConnected(browser_id) => {
                            info!("Browser connected: {}", browser_id);
                            app_state.browser_count += 1;
                        }
                        UiEvent::BrowserDisconnected(browser_id) => {
                            info!("Browser disconnected: {}", browser_id);
                            app_state.browser_count = app_state.browser_count.saturating_sub(1);
                        }
                        UiEvent::TunnelUrl(url) => {
                            info!("Tunnel URL: {}", url);
                            app_state.tunnel_url = Some(url);
                            app_state.update_url_display();
                        }
                        UiEvent::RelayError(msg) => {
                            error!("Relay error: {}", msg);
                        }
                        UiEvent::ShellConnected { session_id, name } => {
                            info!("Shell connected: {} ({})", name, session_id);
                            app_state.shell_count += 1;
                            app_state.update_count_display();
                        }
                        UiEvent::ShellDisconnected { session_id } => {
                            info!("Shell disconnected: {}", session_id);
                            app_state.shell_count = app_state.shell_count.saturating_sub(1);
                            app_state.update_count_display();
                        }
                        UiEvent::ShellRenamed { session_id, name } => {
                            info!("Shell renamed: {} -> {}", session_id, name);
                        }
                        UiEvent::ShellCountChanged(count) => {
                            debug!("Shell count changed: {}", count);
                            app_state.shell_count = count;
                            app_state.update_count_display();
                        }
                        UiEvent::PtyError(msg) => {
                            error!("PTY error: {}", msg);
                        }
                        UiEvent::TerminalDataFromShell { session_id, data } => {
                            debug!(
                                "Terminal data from shell {}: {} bytes",
                                session_id,
                                data.len()
                            );
                        }
                        UiEvent::TerminalDataFromRelay { session_id, data } => {
                            debug!(
                                "Terminal data from relay {}: {} bytes",
                                session_id,
                                data.len()
                            );
                        }
                    }
                }
            }
        }

        // Reset copy button text after 2 seconds
        if let Some(reset_time) = self.copy_reset_time {
            if Instant::now() >= reset_time {
                if let Some(app_state) = &self.app_state {
                    app_state.copy_item.set_text("Copy URL");
                }
                self.copy_reset_time = None;
            }
        }
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // This is called when the app becomes active
        // On macOS this is called once at startup
        debug!("Application resumed");
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        // Set up polling - we need to check our channels periodically
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16), // ~60fps
        ));

        // Handle wake-ups and polls
        if matches!(
            cause,
            winit::event::StartCause::Poll
                | winit::event::StartCause::ResumeTimeReached { .. }
                | winit::event::StartCause::WaitCancelled { .. }
        ) {
            self.handle_ui_events();
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::TrayIconEvent(e) => {
                debug!("Tray event: {:?}", e);
            }
            AppEvent::MenuEvent(e) => {
                self.handle_menu_event(e);
            }
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _event: WindowEvent,
    ) {
        // We don't have any windows, but winit requires this
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Called right before the event loop goes to sleep
        // Good time to check our channels one more time
        self.handle_ui_events();
    }
}

fn main() {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("Starting mac-client menu bar application");

    // Create the event loop FIRST (required on macOS)
    let event_loop = EventLoop::<AppEvent>::with_user_event()
        .build()
        .expect("Failed to create event loop");

    // Set up event handlers to forward events to our event loop
    let proxy = event_loop.create_proxy();
    tray_icon::TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(AppEvent::TrayIconEvent(event));
    }));

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(AppEvent::MenuEvent(event));
    }));

    // Create channels for UI <-> background communication
    let (ui_tx, ui_rx) = mpsc::channel::<UiEvent>();
    let (bg_tx, bg_rx) = mpsc::channel::<BackgroundCommand>();

    // Create pty command channel (sender stays in main thread)
    let (pty_cmd_tx, pty_cmd_rx) = tokio::sync::mpsc::unbounded_channel::<PtyCommand>();

    // Shared PID for killing cloudflared on quit
    let cloudflared_pid = Arc::new(AtomicU32::new(0));

    // Spawn background thread with Tokio runtime
    let ui_tx_bg = ui_tx.clone();
    let cloudflared_pid_bg = cloudflared_pid.clone();
    let bg_handle = thread::spawn(move || {
        run_background_tasks(ui_tx_bg, bg_rx, pty_cmd_rx, cloudflared_pid_bg);
    });

    // Load icon from embedded bytes
    let icon_bytes = include_bytes!("../resources/icon.png");
    let icon_image = ImageReader::new(Cursor::new(icon_bytes))
        .with_guessed_format()
        .expect("Failed to read icon format")
        .decode()
        .expect("Failed to decode icon");
    let icon_rgba = icon_image.to_rgba8();
    let (width, height) = icon_rgba.dimensions();
    let icon = tray_icon::Icon::from_rgba(icon_rgba.into_raw(), width, height)
        .expect("Failed to create icon");

    debug!("Icon loaded: {}x{}", width, height);

    // Build the menu
    let menu = Menu::new();

    // Status display items (disabled - for display only)
    let url_item = MenuItem::new("URL: starting tunnel...", false, None);
    let code_item = MenuItem::new("Code: ------", false, None);
    let status_item = MenuItem::new("Status: Connecting...", false, None);
    let sessions_item = MenuItem::new("Sessions: 0", false, None);

    // Action items
    let regen_code_item = MenuItem::with_id(ID_REGEN_CODE, "Regenerate Code", true, None);
    let copy_url_item = MenuItem::with_id(ID_COPY_URL, "Copy URL", true, None);
    let copy_code_item = MenuItem::with_id(ID_COPY_CODE, "Copy Session Code", true, None);

    // Check current login item status and set initial checkbox state
    let is_login_enabled = is_login_item_enabled();
    let login_item =
        CheckMenuItem::with_id(ID_LOGIN_ITEM, "Start at Login", true, is_login_enabled, None);
    debug!("Login item initial state: {}", is_login_enabled);

    let quit_item = MenuItem::with_id(ID_QUIT, "Quit", true, None);

    // Assemble menu
    menu.append(&url_item).expect("Failed to add url item");
    menu.append(&code_item).expect("Failed to add code item");
    menu.append(&status_item)
        .expect("Failed to add status item");
    menu.append(&sessions_item)
        .expect("Failed to add sessions item");
    menu.append(&PredefinedMenuItem::separator())
        .expect("Failed to add separator");
    menu.append(&copy_url_item)
        .expect("Failed to add copy url item");
    menu.append(&copy_code_item)
        .expect("Failed to add copy code item");
    menu.append(&regen_code_item)
        .expect("Failed to add regen code item");
    menu.append(&PredefinedMenuItem::separator())
        .expect("Failed to add separator");
    menu.append(&login_item)
        .expect("Failed to add login item");
    menu.append(&PredefinedMenuItem::separator())
        .expect("Failed to add separator");
    menu.append(&quit_item).expect("Failed to add quit item");

    debug!("Menu constructed with {} items", 9);

    // Create app state with menu item references
    let app_state = AppState::new(
        code_item,
        status_item,
        sessions_item,
        url_item,
        copy_url_item.clone(),
    );

    // Create tray icon
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_icon_as_template(true)
        .with_tooltip("Terminal Remote")
        .build()
        .expect("Failed to create tray icon");

    info!("Tray icon created successfully");

    // Create our application handler
    let mut app = App::new();
    app.tray_icon = Some(tray_icon);
    app.app_state = Some(app_state);
    app.login_item = Some(login_item);
    app.bg_tx = Some(bg_tx);
    app.ui_rx = Some(ui_rx);
    app.bg_handle = Some(bg_handle);
    app.pty_cmd_tx = Some(pty_cmd_tx);
    app.cloudflared_pid = cloudflared_pid;

    info!("Entering main event loop");

    // Run the event loop - this blocks until the app exits
    event_loop.run_app(&mut app).expect("Event loop failed");

    // Clean up
    info!("Waiting for background thread to finish...");
    if let Some(handle) = app.bg_handle.take() {
        if let Err(e) = handle.join() {
            error!("Background thread panicked: {:?}", e);
        }
    }

    info!("Application exiting");
}

/// Check if the app is currently registered as a login item.
///
/// Returns true if enabled, false otherwise (not registered, requires approval, or not found).
fn is_login_item_enabled() -> bool {
    let service = AppService::new(ServiceType::MainApp);
    matches!(service.status(), ServiceStatus::Enabled)
}

/// Configure the app as a login item (register or unregister).
///
/// This uses SMAppService which requires:
/// - macOS 13.0+
/// - App must be properly signed and bundled
/// - May require user approval in System Settings > Login Items
fn configure_login_item(enable: bool) -> Result<(), Box<dyn std::error::Error>> {
    let service = AppService::new(ServiceType::MainApp);

    if enable {
        service.register()?;
        info!("Registered as login item");

        // Check if user approval is needed
        if matches!(service.status(), ServiceStatus::RequiresApproval) {
            info!("Login item requires user approval in System Settings > Login Items");
            // Optionally open System Settings
            AppService::open_system_settings_login_items();
        }
    } else {
        service.unregister()?;
        info!("Unregistered as login item");
    }

    Ok(())
}

/// Run background tasks (relay client and PTY manager) on a Tokio runtime.
fn run_background_tasks(
    ui_tx: mpsc::Sender<UiEvent>,
    bg_rx: mpsc::Receiver<BackgroundCommand>,
    pty_cmd_rx: tokio::sync::mpsc::UnboundedReceiver<PtyCommand>,
    cloudflared_pid: Arc<AtomicU32>,
) {
    info!("Background thread starting");

    let rt = Runtime::new().expect("Failed to create Tokio runtime");

    rt.block_on(async {
        // Get relay URL from env or default
        let relay_url = std::env::var("RELAY_URL")
            .unwrap_or_else(|_| "ws://localhost:3000/ws".to_string());
        info!("Using relay URL: {}", relay_url);

        // Create channels for relay events
        let (relay_event_tx, relay_event_rx) = mpsc::channel::<RelayEvent>();

        // Create relay command channel
        let (relay_cmd_tx, relay_cmd_rx) = tokio::sync::mpsc::unbounded_channel::<RelayCommand>();

        // Create relay client
        let mut relay = RelayClient::new(relay_url, relay_event_tx, relay_cmd_rx);

        // Store command senders for data forwarding
        let relay_cmd_tx_for_pty = relay_cmd_tx.clone();
        let relay_cmd_tx_for_relay = relay_cmd_tx.clone();

        // Shared session list for browser sync
        let session_list: std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let session_list_for_pty = session_list.clone();
        let session_list_for_relay = session_list.clone();

        // Create PTY manager (replaces both TmuxManager and IpcServer)
        let (_pty_manager, mut pty_event_rx, pty_internal_cmd_tx) = PtyManager::new();

        // No AttachAll needed — sessions auto-register when pty-proxy connects

        // Clone for relay forwarding (before move)
        let pty_cmd_tx_for_relay = pty_internal_cmd_tx.clone();

        // Forward pty commands from main thread to pty manager
        let mut pty_cmd_rx = pty_cmd_rx;
        let pty_forward_handle = tokio::spawn(async move {
            while let Some(cmd) = pty_cmd_rx.recv().await {
                if pty_internal_cmd_tx.send(cmd).is_err() {
                    break;
                }
            }
        });

        // Spawn cloudflared tunnel
        let ui_tx_tunnel = ui_tx.clone();
        let cloudflared_pid = cloudflared_pid.clone();
        let tunnel_handle = tokio::task::spawn_blocking(move || {
            run_cloudflared_tunnel(ui_tx_tunnel, cloudflared_pid);
        });

        // Forward PTY events to relay (output -> browser)
        let ui_tx_pty = ui_tx.clone();
        let pty_event_handle = tokio::spawn(async move {
            while let Some(event) = pty_event_rx.recv().await {
                match event {
                    PtyEvent::Attached { session_id, session_name } => {
                        info!("pty-proxy session connected: {} ({})", session_name, session_id);
                        // Update session list
                        {
                            let mut list = session_list_for_pty.lock().unwrap();
                            list.push((session_id.clone(), session_name.clone()));
                        }
                        // Notify relay to send to browser
                        let _ = relay_cmd_tx_for_pty.send(RelayCommand::SendSessionConnected {
                            session_id: session_id.clone(),
                            name: session_name.clone(),
                        });
                        // Notify UI
                        let _ = ui_tx_pty.send(UiEvent::ShellConnected {
                            session_id,
                            name: session_name,
                        });
                    }
                    PtyEvent::Detached { session_id } => {
                        info!("pty-proxy session disconnected: {}", session_id);
                        // Update session list
                        {
                            let mut list = session_list_for_pty.lock().unwrap();
                            list.retain(|(id, _)| id != &session_id);
                        }
                        // Notify relay to send to browser
                        let _ = relay_cmd_tx_for_pty.send(RelayCommand::SendSessionDisconnected {
                            session_id: session_id.clone(),
                        });
                        // Notify UI
                        let _ = ui_tx_pty.send(UiEvent::ShellDisconnected { session_id });
                    }
                    PtyEvent::Output { session_id, data } => {
                        // Forward pty output to relay for browser
                        let _ = relay_cmd_tx_for_pty.send(RelayCommand::SendTerminalData {
                            session_id,
                            data,
                        });
                    }
                    PtyEvent::SessionResize { session_id, cols, rows } => {
                        // Forward mac terminal resize to browser (one-way: mac -> UI)
                        let _ = relay_cmd_tx_for_pty.send(RelayCommand::SendSessionResize {
                            session_id,
                            cols,
                            rows,
                        });
                    }
                    PtyEvent::Error(msg) => {
                        error!("PTY error: {}", msg);
                    }
                }
            }
        });

        // Spawn relay client task
        let relay_handle = tokio::spawn(async move {
            relay.run().await;
        });

        // Spawn event forwarding task
        let ui_tx_relay = ui_tx.clone();
        let relay_forward_handle = tokio::task::spawn_blocking(move || {
            forward_relay_events(
                relay_event_rx,
                ui_tx_relay,
                pty_cmd_tx_for_relay,
                relay_cmd_tx_for_relay,
                session_list_for_relay,
            );
        });

        // Wait for shutdown signal
        loop {
            match bg_rx.try_recv() {
                Ok(BackgroundCommand::Shutdown) => {
                    info!("Shutdown command received");
                    break;
                }
                Ok(BackgroundCommand::SendTerminalData { session_id, data }) => {
                    // Forward terminal data to relay
                    let _ = relay_cmd_tx.send(RelayCommand::SendTerminalData { session_id, data });
                }
                Ok(BackgroundCommand::SendToShell { session_id, data }) => {
                    // Forward terminal data to shell via PTY manager
                    let _ = relay_cmd_tx.send(RelayCommand::SendTerminalData { session_id, data });
                }
                Ok(BackgroundCommand::ReconnectRelay) => {
                    info!("Reconnecting relay to regenerate session code");
                    let _ = relay_cmd_tx.send(RelayCommand::Reconnect);
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // No command, continue
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    info!("Background command channel disconnected, shutting down");
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Abort tasks (they run forever, so we need to abort them)
        relay_handle.abort();
        relay_forward_handle.abort();
        pty_forward_handle.abort();
        pty_event_handle.abort();
        tunnel_handle.abort();

        info!("Background tasks shut down");
    });

    info!("Background thread exiting");
}


/// Forward relay events to the UI channel.
///
/// This runs in a spawn_blocking task because std::sync::mpsc::recv() is blocking.
/// Converts RelayEvent from the relay module into UiEvent for the main thread.
/// Also forwards terminal data from relay to PTY manager (browser -> shell).
fn forward_relay_events(
    rx: mpsc::Receiver<RelayEvent>,
    ui_tx: mpsc::Sender<UiEvent>,
    pty_cmd_tx: tokio::sync::mpsc::UnboundedSender<PtyCommand>,
    relay_cmd_tx: tokio::sync::mpsc::UnboundedSender<RelayCommand>,
    session_list: std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>,
) {
    debug!("Relay event forwarder starting");
    loop {
        match rx.recv() {
            Ok(event) => {
                let ui_event = match event {
                    RelayEvent::Connected => UiEvent::RelayConnected,
                    RelayEvent::Disconnected => UiEvent::RelayDisconnected,
                    RelayEvent::SessionCode(code) => UiEvent::SessionCode(code),
                    RelayEvent::BrowserConnected(id) => {
                        // Send session list to newly connected browser
                        let sessions = session_list.lock().unwrap().clone();
                        info!("Browser connected, sending {} sessions", sessions.len());
                        let _ = relay_cmd_tx.send(RelayCommand::SendSessionList { sessions });
                        UiEvent::BrowserConnected(id)
                    }
                    RelayEvent::BrowserDisconnected(id) => UiEvent::BrowserDisconnected(id),
                    RelayEvent::Error(msg) => UiEvent::RelayError(msg),
                    RelayEvent::TerminalData { session_id, data } => {
                        // Forward to PTY manager (browser -> shell)
                        let _ = pty_cmd_tx.send(PtyCommand::Write {
                            session_id: session_id.clone(),
                            data: data.clone(),
                        });
                        UiEvent::TerminalDataFromRelay { session_id, data }
                    }
                    RelayEvent::CloseSession { session_id } => {
                        // Kill the pty-proxy session
                        info!("Closing session: {}", session_id);
                        let _ = pty_cmd_tx.send(PtyCommand::KillSession {
                            session_id: session_id.clone(),
                        });
                        // No UI event - session will emit Detached event
                        continue;
                    }
                    RelayEvent::CreateSession => {
                        info!("Creating new terminal session");
                        match std::process::Command::new("osascript")
                            .arg("-e")
                            .arg(r#"tell application "Terminal"
    activate
    do script ""
end tell"#)
                            .output()
                        {
                            Ok(output) => {
                                if output.status.success() {
                                    info!("New terminal window created");
                                } else {
                                    error!(
                                        "osascript create failed ({}): {}",
                                        output.status,
                                        String::from_utf8_lossy(&output.stderr)
                                    );
                                }
                            }
                            Err(e) => {
                                error!("Failed to run osascript for create: {}", e);
                            }
                        }
                        continue;
                    }
                };
                if ui_tx.send(ui_event).is_err() {
                    debug!("UI channel closed, stopping relay event forwarding");
                    break;
                }
            }
            Err(_) => {
                debug!("Relay event channel closed");
                break;
            }
        }
    }
    debug!("Relay event forwarder exiting");
}

/// Spawn cloudflared tunnel and parse the URL from stderr.
///
/// Returns the child process handle so it can be killed on shutdown.
/// Sends UiEvent::TunnelUrl when the tunnel URL is found.
/// Find cloudflared binary, checking Homebrew paths first.
fn find_cloudflared() -> String {
    for path in &[
        "/opt/homebrew/bin/cloudflared",
        "/usr/local/bin/cloudflared",
    ] {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    // Fall back to PATH lookup
    "cloudflared".to_string()
}

fn run_cloudflared_tunnel(ui_tx: mpsc::Sender<UiEvent>, pid_store: Arc<AtomicU32>) -> Option<Child> {
    let cloudflared = find_cloudflared();
    info!("Using cloudflared at: {}", cloudflared);

    let mut child = match Command::new(&cloudflared)
        .args(["tunnel", "--url", "http://localhost:3000"])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            error!("Failed to spawn cloudflared: {}", e);
            let _ = ui_tx.send(UiEvent::RelayError(format!("cloudflared not found: {}", e)));
            return None;
        }
    };

    let child_pid = child.id();
    info!("cloudflared tunnel started (pid {})", child_pid);
    pid_store.store(child_pid, Ordering::Relaxed);

    let stderr = child.stderr.take().expect("stderr was piped");
    let reader = BufReader::new(stderr);

    for line in reader.lines() {
        match line {
            Ok(line) => {
                // Look for the tunnel URL in cloudflared output
                if let Some(url) = extract_tunnel_url(&line) {
                    info!("Tunnel URL found: {}", url);
                    let _ = ui_tx.send(UiEvent::TunnelUrl(url));
                } else {
                    debug!("cloudflared: {}", line);
                }
            }
            Err(e) => {
                error!("Error reading cloudflared stderr: {}", e);
                break;
            }
        }
    }

    // Process exited (stderr closed)
    match child.wait() {
        Ok(status) => warn!("cloudflared exited with status: {}", status),
        Err(e) => error!("Error waiting for cloudflared: {}", e),
    }

    None
}

/// Extract a trycloudflare.com URL from a log line.
fn extract_tunnel_url(line: &str) -> Option<String> {
    // cloudflared prints the URL in a line like:
    // ... | https://something-something.trycloudflare.com
    for word in line.split_whitespace() {
        if word.starts_with("https://") && word.contains("trycloudflare.com") {
            return Some(word.to_string());
        }
    }
    None
}
