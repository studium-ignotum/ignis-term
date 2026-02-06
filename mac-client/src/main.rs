//! Mac Client - Menu Bar Application
//!
//! A macOS menu bar application for managing remote terminal sessions.
//! This module integrates the tray icon, relay client, and IPC server.
//!
//! On macOS, we must use a proper event loop for the tray icon to appear.
//! We use winit's EventLoop to drive the main thread.

use image::ImageReader;
use mac_client::app::{AppState, BackgroundCommand, UiEvent};
use mac_client::ipc::{IpcCommand, IpcEvent, IpcServer};
use mac_client::relay::{RelayClient, RelayCommand, RelayEvent};
use muda::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use smappservice_rs::{AppService, ServiceStatus, ServiceType};
use std::io::Cursor;
use std::sync::mpsc;
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
        }
    }

    fn handle_menu_event(&mut self, event: muda::MenuEvent, event_loop: &ActiveEventLoop) {
        debug!("Menu event: {:?}", event);

        match event.id().0.as_str() {
            ID_COPY_CODE => {
                if let Some(app_state) = &self.app_state {
                    if let Some(code) = &app_state.session_code {
                        match arboard::Clipboard::new() {
                            Ok(mut clipboard) => {
                                if let Err(e) = clipboard.set_text(code.clone()) {
                                    error!("Failed to set clipboard: {}", e);
                                } else {
                                    info!("Session code copied to clipboard: {}", code);
                                    if let Some(app_state) = &self.app_state {
                                        app_state.copy_item.set_text("Copied!");
                                    }
                                    self.copy_reset_time =
                                        Some(Instant::now() + Duration::from_secs(2));
                                }
                            }
                            Err(e) => {
                                error!("Failed to access clipboard: {}", e);
                            }
                        }
                    } else {
                        warn!("Copy requested but no session code available");
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
                info!("Quit requested, shutting down...");
                if let Some(bg_tx) = &self.bg_tx {
                    let _ = bg_tx.send(BackgroundCommand::Shutdown);
                }
                event_loop.exit();
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
                        UiEvent::IpcError(msg) => {
                            error!("IPC error: {}", msg);
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
                    app_state.copy_item.set_text("Copy Session Code");
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

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::TrayIconEvent(e) => {
                debug!("Tray event: {:?}", e);
            }
            AppEvent::MenuEvent(e) => {
                self.handle_menu_event(e, event_loop);
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

    // Spawn background thread with Tokio runtime
    let ui_tx_bg = ui_tx.clone();
    let bg_handle = thread::spawn(move || {
        run_background_tasks(ui_tx_bg, bg_rx);
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
    let code_item = MenuItem::new("Code: ------", false, None);
    let status_item = MenuItem::new("Status: Connecting...", false, None);
    let sessions_item = MenuItem::new("Sessions: 0", false, None);

    // Action items
    let copy_code_item = MenuItem::with_id(ID_COPY_CODE, "Copy Session Code", true, None);

    // Check current login item status and set initial checkbox state
    let is_login_enabled = is_login_item_enabled();
    let login_item =
        CheckMenuItem::with_id(ID_LOGIN_ITEM, "Start at Login", true, is_login_enabled, None);
    debug!("Login item initial state: {}", is_login_enabled);

    let quit_item = MenuItem::with_id(ID_QUIT, "Quit", true, None);

    // Assemble menu
    menu.append(&code_item).expect("Failed to add code item");
    menu.append(&status_item)
        .expect("Failed to add status item");
    menu.append(&sessions_item)
        .expect("Failed to add sessions item");
    menu.append(&PredefinedMenuItem::separator())
        .expect("Failed to add separator");
    menu.append(&copy_code_item)
        .expect("Failed to add copy item");
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
        copy_code_item.clone(),
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

/// Run background tasks (relay client and IPC server) on a Tokio runtime.
fn run_background_tasks(ui_tx: mpsc::Sender<UiEvent>, bg_rx: mpsc::Receiver<BackgroundCommand>) {
    info!("Background thread starting");

    let rt = Runtime::new().expect("Failed to create Tokio runtime");

    rt.block_on(async {
        // Get relay URL from env or default
        let relay_url = std::env::var("RELAY_URL")
            .unwrap_or_else(|_| "ws://localhost:3000/ws".to_string());
        info!("Using relay URL: {}", relay_url);

        // Create channels for module events
        let (relay_event_tx, relay_event_rx) = mpsc::channel::<RelayEvent>();
        let (ipc_event_tx, ipc_event_rx) = mpsc::channel::<IpcEvent>();

        // Create command channels
        let (relay_cmd_tx, relay_cmd_rx) = tokio::sync::mpsc::unbounded_channel::<RelayCommand>();
        let (ipc_cmd_tx, ipc_cmd_rx) = tokio::sync::mpsc::unbounded_channel::<IpcCommand>();

        // Create relay client
        let mut relay = RelayClient::new(relay_url, relay_event_tx, relay_cmd_rx);

        // Store command senders for data forwarding
        let relay_cmd_tx_for_ipc = relay_cmd_tx.clone();
        let ipc_cmd_tx_for_relay = ipc_cmd_tx.clone();

        // Create IPC server
        let mut ipc = match IpcServer::new(ipc_event_tx, ipc_cmd_rx).await {
            Ok(server) => server,
            Err(e) => {
                error!("Failed to create IPC server: {}", e);
                let _ = ui_tx.send(UiEvent::IpcError(format!("Failed to start IPC: {}", e)));
                // Continue without IPC - relay still works
                return run_relay_only(relay, ui_tx, bg_rx).await;
            }
        };

        // Spawn relay client task
        let relay_handle = tokio::spawn(async move {
            relay.run().await;
        });

        // Spawn IPC server task
        let ipc_handle = tokio::spawn(async move {
            ipc.run().await;
        });

        // Spawn event forwarding tasks
        let ui_tx_relay = ui_tx.clone();
        let relay_forward_handle = tokio::task::spawn_blocking(move || {
            forward_relay_events(relay_event_rx, ui_tx_relay, ipc_cmd_tx_for_relay);
        });

        let ui_tx_ipc = ui_tx.clone();
        let ipc_forward_handle = tokio::task::spawn_blocking(move || {
            forward_ipc_events(ipc_event_rx, ui_tx_ipc, relay_cmd_tx_for_ipc);
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
                    // Forward terminal data to shell via IPC
                    let _ = ipc_cmd_tx.send(IpcCommand::WriteToSession { session_id, data });
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
        ipc_handle.abort();
        relay_forward_handle.abort();
        ipc_forward_handle.abort();

        info!("Background tasks shut down");
    });

    info!("Background thread exiting");
}

/// Run only the relay client (fallback if IPC fails to start).
async fn run_relay_only(
    _relay: RelayClient,
    ui_tx: mpsc::Sender<UiEvent>,
    bg_rx: mpsc::Receiver<BackgroundCommand>,
) {
    let (relay_event_tx, relay_event_rx) = mpsc::channel::<RelayEvent>();
    let (relay_cmd_tx, relay_cmd_rx) = tokio::sync::mpsc::unbounded_channel::<RelayCommand>();

    // Create a dummy IPC command sender (unused in relay-only mode)
    let (dummy_ipc_tx, _dummy_ipc_rx) = tokio::sync::mpsc::unbounded_channel::<IpcCommand>();

    // Create a new relay with the fresh channel
    let relay_url = std::env::var("RELAY_URL")
        .unwrap_or_else(|_| "ws://localhost:3000/ws".to_string());
    let mut relay = RelayClient::new(relay_url, relay_event_tx, relay_cmd_rx);

    let relay_handle = tokio::spawn(async move {
        relay.run().await;
    });

    let ui_tx_relay = ui_tx.clone();
    let relay_forward_handle = tokio::task::spawn_blocking(move || {
        forward_relay_events(relay_event_rx, ui_tx_relay, dummy_ipc_tx);
    });

    // Wait for shutdown
    loop {
        match bg_rx.try_recv() {
            Ok(BackgroundCommand::Shutdown) => break,
            Ok(BackgroundCommand::SendTerminalData { session_id, data }) => {
                let _ = relay_cmd_tx.send(RelayCommand::SendTerminalData { session_id, data });
            }
            Ok(BackgroundCommand::SendToShell { .. }) => {
                // IPC not available in relay-only mode
                warn!("Cannot send to shell: IPC not available");
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => {}
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    relay_handle.abort();
    relay_forward_handle.abort();
}

/// Forward relay events to the UI channel.
///
/// This runs in a spawn_blocking task because std::sync::mpsc::recv() is blocking.
/// Converts RelayEvent from the relay module into UiEvent for the main thread.
/// Also forwards terminal data from relay to IPC (browser -> shell).
fn forward_relay_events(
    rx: mpsc::Receiver<RelayEvent>,
    ui_tx: mpsc::Sender<UiEvent>,
    ipc_cmd_tx: tokio::sync::mpsc::UnboundedSender<IpcCommand>,
) {
    debug!("Relay event forwarder starting");
    loop {
        match rx.recv() {
            Ok(event) => {
                let ui_event = match event {
                    RelayEvent::Connected => UiEvent::RelayConnected,
                    RelayEvent::Disconnected => UiEvent::RelayDisconnected,
                    RelayEvent::SessionCode(code) => UiEvent::SessionCode(code),
                    RelayEvent::BrowserConnected(id) => UiEvent::BrowserConnected(id),
                    RelayEvent::BrowserDisconnected(id) => UiEvent::BrowserDisconnected(id),
                    RelayEvent::Error(msg) => UiEvent::RelayError(msg),
                    RelayEvent::TerminalData { session_id, data } => {
                        // Forward to IPC for shell
                        let _ = ipc_cmd_tx.send(IpcCommand::WriteToSession {
                            session_id: session_id.clone(),
                            data: data.clone(),
                        });
                        UiEvent::TerminalDataFromRelay { session_id, data }
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

/// Forward IPC events to the UI channel.
///
/// This runs in a spawn_blocking task because std::sync::mpsc::recv() is blocking.
/// Converts IpcEvent from the ipc module into UiEvent for the main thread.
/// Also forwards terminal data from IPC to relay (shell -> browser).
fn forward_ipc_events(
    rx: mpsc::Receiver<IpcEvent>,
    ui_tx: mpsc::Sender<UiEvent>,
    relay_cmd_tx: tokio::sync::mpsc::UnboundedSender<RelayCommand>,
) {
    debug!("IPC event forwarder starting");
    loop {
        match rx.recv() {
            Ok(event) => {
                let ui_event = match event {
                    IpcEvent::SessionConnected { session_id, name } => {
                        UiEvent::ShellConnected { session_id, name }
                    }
                    IpcEvent::SessionDisconnected { session_id } => {
                        UiEvent::ShellDisconnected { session_id }
                    }
                    IpcEvent::SessionRenamed { session_id, name } => {
                        UiEvent::ShellRenamed { session_id, name }
                    }
                    IpcEvent::SessionCountChanged(count) => UiEvent::ShellCountChanged(count),
                    IpcEvent::TerminalData { session_id, data } => {
                        // Forward to relay for browser
                        let _ = relay_cmd_tx.send(RelayCommand::SendTerminalData {
                            session_id: session_id.clone(),
                            data: data.clone(),
                        });
                        UiEvent::TerminalDataFromShell { session_id, data }
                    }
                    IpcEvent::Error(msg) => UiEvent::IpcError(msg),
                };
                if ui_tx.send(ui_event).is_err() {
                    debug!("UI channel closed, stopping IPC event forwarding");
                    break;
                }
            }
            Err(_) => {
                debug!("IPC event channel closed");
                break;
            }
        }
    }
    debug!("IPC event forwarder exiting");
}
