//! Mac Client - Menu Bar Application
//!
//! A macOS menu bar application for managing remote terminal sessions.
//! This module sets up the tray icon and handles user interactions.

use image::ImageReader;
use muda::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use std::io::Cursor;
use std::thread;
use std::time::Duration;
use tray_icon::{TrayIconBuilder, TrayIconEvent};
use tracing::{debug, info};

// Channel message types for UI <-> background communication (future use)
#[derive(Debug)]
#[allow(dead_code)]
pub enum UiCommand {
    /// Request to copy session code to clipboard
    CopyCode,
    /// Toggle auto-start at login
    ToggleLoginItem(bool),
    /// Request application quit
    Quit,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum BackgroundEvent {
    /// Connection status changed
    ConnectionStatus(String),
    /// Session code updated
    SessionCode(String),
    /// Active session count changed
    SessionCount(u32),
}

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
    menu.append(&status_item).expect("Failed to add status item");
    menu.append(&sessions_item).expect("Failed to add sessions item");
    menu.append(&PredefinedMenuItem::separator()).expect("Failed to add separator");
    menu.append(&copy_code_item).expect("Failed to add copy item");
    menu.append(&PredefinedMenuItem::separator()).expect("Failed to add separator");
    menu.append(&login_item).expect("Failed to add login item");
    menu.append(&PredefinedMenuItem::separator()).expect("Failed to add separator");
    menu.append(&quit_item).expect("Failed to add quit item");

    debug!("Menu constructed with {} items", 9);

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

    // Main event loop
    info!("Entering main event loop");
    loop {
        // Poll menu events
        if let Ok(event) = menu_receiver.try_recv() {
            debug!("Menu event: {:?}", event);

            match event.id().0.as_str() {
                ID_COPY_CODE => {
                    info!("Copy session code requested (placeholder)");
                    // TODO: Implement actual copy functionality in integration plan
                }
                ID_LOGIN_ITEM => {
                    info!("Login item toggled (placeholder)");
                    // TODO: Implement login item functionality in Plan 05-05
                }
                ID_QUIT => {
                    info!("Quit requested, exiting...");
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

        // Small sleep to avoid busy-waiting
        thread::sleep(Duration::from_millis(10));
    }

    info!("Application exiting");
}
