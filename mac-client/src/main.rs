//! Mac Client - Menu Bar Application
//!
//! A macOS menu bar application for managing remote terminal sessions.
//! This module integrates the tray icon, relay client, and IPC server.

use image::ImageReader;
use mac_client::app::{AppState, BackgroundCommand, UiEvent};
use mac_client::ipc::{IpcEvent, IpcServer};
use mac_client::relay::{RelayClient, RelayCommand, RelayEvent};
use muda::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use std::io::Cursor;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tray_icon::{TrayIconBuilder, TrayIconEvent};
use tracing::{debug, error, info, warn};

// Menu item IDs
const ID_COPY_CODE: &str = "copy_code";
const ID_LOGIN_ITEM: &str = "login_item";
const ID_QUIT: &str = "quit";

fn main() {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("Starting mac-client menu bar application");

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
    let login_item = CheckMenuItem::with_id(ID_LOGIN_ITEM, "Start at Login", true, false, None);
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
    let mut app_state = AppState::new(
        code_item,
        status_item,
        sessions_item,
        copy_code_item.clone(),
    );

    // Create tray icon
    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_icon_as_template(true)
        .with_tooltip("Terminal Remote")
        .build()
        .expect("Failed to create tray icon");

    info!("Tray icon created successfully");

    // Get event receivers
    let menu_receiver = MenuEvent::receiver();
    let tray_receiver = TrayIconEvent::receiver();

    // Track when to reset copy button text
    let mut copy_reset_time: Option<Instant> = None;

    // Main event loop
    info!("Entering main event loop");
    loop {
        // Poll menu events
        if let Ok(event) = menu_receiver.try_recv() {
            debug!("Menu event: {:?}", event);

            match event.id().0.as_str() {
                ID_COPY_CODE => {
                    if let Some(code) = &app_state.session_code {
                        match arboard::Clipboard::new() {
                            Ok(mut clipboard) => {
                                if let Err(e) = clipboard.set_text(code.clone()) {
                                    error!("Failed to set clipboard: {}", e);
                                } else {
                                    info!("Session code copied to clipboard: {}", code);

                                    // Show confirmation by changing menu item text
                                    app_state.copy_item.set_text("Copied!");
                                    copy_reset_time =
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
                ID_LOGIN_ITEM => {
                    info!("Login item toggled (placeholder)");
                    // TODO: Implement login item functionality in Plan 05-05
                }
                ID_QUIT => {
                    info!("Quit requested, shutting down...");
                    let _ = bg_tx.send(BackgroundCommand::Shutdown);
                    break;
                }
                _ => {
                    debug!("Unknown menu item clicked: {:?}", event.id());
                }
            }
        }

        // Poll tray icon events
        if let Ok(event) = tray_receiver.try_recv() {
            debug!("Tray event: {:?}", event);
        }

        // Reset copy button text after 2 seconds
        if let Some(reset_time) = copy_reset_time {
            if Instant::now() >= reset_time {
                app_state.copy_item.set_text("Copy Session Code");
                copy_reset_time = None;
            }
        }

        // Handle UI events from background
        while let Ok(event) = ui_rx.try_recv() {
            debug!("UI event: {:?}", event);

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
                UiEvent::ShellCountChanged(count) => {
                    debug!("Shell count changed: {}", count);
                    app_state.shell_count = count;
                    app_state.update_count_display();
                }
                UiEvent::IpcError(msg) => {
                    error!("IPC error: {}", msg);
                }
                UiEvent::TerminalDataFromShell { session_id, data } => {
                    // Terminal data forwarding handled in background thread
                    debug!(
                        "Terminal data from shell {}: {} bytes",
                        session_id,
                        data.len()
                    );
                }
                UiEvent::TerminalDataFromRelay { session_id, data } => {
                    // Terminal data forwarding handled in background thread
                    debug!(
                        "Terminal data from relay {}: {} bytes",
                        session_id,
                        data.len()
                    );
                }
            }
        }

        // Small sleep to avoid busy-waiting
        thread::sleep(Duration::from_millis(10));
    }

    // Wait for background thread to finish
    info!("Waiting for background thread to finish...");
    if let Err(e) = bg_handle.join() {
        error!("Background thread panicked: {:?}", e);
    }

    info!("Application exiting");
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

        // Create command channel for sending data to relay
        let (relay_cmd_tx, relay_cmd_rx) = tokio::sync::mpsc::unbounded_channel::<RelayCommand>();

        // Create relay client
        let mut relay = RelayClient::new(relay_url, relay_event_tx, relay_cmd_rx);

        // Store command sender for use by IPC data forwarding
        let _relay_cmd_tx = relay_cmd_tx;

        // Create IPC server
        let mut ipc = match IpcServer::new(ipc_event_tx).await {
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
            forward_relay_events(relay_event_rx, ui_tx_relay);
        });

        let ui_tx_ipc = ui_tx.clone();
        let ipc_forward_handle = tokio::task::spawn_blocking(move || {
            forward_ipc_events(ipc_event_rx, ui_tx_ipc);
        });

        // Wait for shutdown signal
        loop {
            match bg_rx.try_recv() {
                Ok(BackgroundCommand::Shutdown) => {
                    info!("Shutdown command received");
                    break;
                }
                Ok(BackgroundCommand::SendTerminalData { .. }) => {
                    // Handled directly by IPC -> relay data forwarding (Task 3)
                }
                Ok(BackgroundCommand::SendToShell { .. }) => {
                    // Handled directly by relay -> IPC data forwarding (Task 3)
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
    let (_relay_cmd_tx, relay_cmd_rx) = tokio::sync::mpsc::unbounded_channel::<RelayCommand>();

    // Create a new relay with the fresh channel
    let relay_url = std::env::var("RELAY_URL")
        .unwrap_or_else(|_| "ws://localhost:3000/ws".to_string());
    let mut relay = RelayClient::new(relay_url, relay_event_tx, relay_cmd_rx);

    let relay_handle = tokio::spawn(async move {
        relay.run().await;
    });

    let ui_tx_relay = ui_tx.clone();
    let relay_forward_handle = tokio::task::spawn_blocking(move || {
        forward_relay_events(relay_event_rx, ui_tx_relay);
    });

    // Wait for shutdown
    loop {
        match bg_rx.try_recv() {
            Ok(BackgroundCommand::Shutdown) => break,
            Ok(BackgroundCommand::SendTerminalData { .. }) => {
                // No IPC server in relay-only mode
            }
            Ok(BackgroundCommand::SendToShell { .. }) => {
                // No IPC server in relay-only mode
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
/// Exits when either the relay event channel closes (sender dropped) or the
/// UI channel closes (main thread exited).
fn forward_relay_events(rx: mpsc::Receiver<RelayEvent>, ui_tx: mpsc::Sender<UiEvent>) {
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
/// Exits when either the IPC event channel closes (sender dropped) or the
/// UI channel closes (main thread exited).
fn forward_ipc_events(rx: mpsc::Receiver<IpcEvent>, ui_tx: mpsc::Sender<UiEvent>) {
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
                    IpcEvent::SessionCountChanged(count) => UiEvent::ShellCountChanged(count),
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
