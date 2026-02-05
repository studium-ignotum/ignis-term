---
phase: 05-mac-client
plan: 05
subsystem: client
tags: [macos, tray-icon, winit, smappservice, login-item, app-bundle]

# Dependency graph
requires:
  - phase: 05-04
    provides: Integrated mac-client with relay and IPC modules
provides:
  - Info.plist for proper .app bundle with LSUIElement (no Dock icon)
  - build-bundle.sh script for creating Terminal Remote.app
  - Login item toggle using SMAppService
  - Winit event loop for proper macOS tray icon rendering
affects: [06-shell-integration, distribution, notarization]

# Tech tracking
tech-stack:
  added: [smappservice-rs 0.1, winit 0.30]
  patterns: [ApplicationHandler trait for macOS event loop, TrayIconEvent::set_event_handler]

key-files:
  created: [mac-client/Info.plist, mac-client/build-bundle.sh]
  modified: [mac-client/Cargo.toml, mac-client/src/main.rs]

key-decisions:
  - "winit event loop required for tray icon to appear on macOS"
  - "ApplicationHandler trait pattern for event-driven macOS apps"
  - "LSUIElement=true hides app from Dock"
  - "SMAppService for login item registration (macOS 13+)"

patterns-established:
  - "EventLoop::with_user_event() for custom event forwarding"
  - "TrayIconEvent::set_event_handler to bridge tray events to winit"
  - "MenuEvent::set_event_handler to bridge menu events to winit"

# Metrics
duration: 45min
completed: 2026-02-06
---

# Phase 5 Plan 5: App Bundle & Login Item Summary

**macOS .app bundle with LSUIElement (no Dock icon), winit event loop for tray rendering, and SMAppService login item toggle**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-02-05T21:39:37Z
- **Completed:** 2026-02-06T04:50:00Z
- **Tasks:** 4 (3 auto + 1 checkpoint)
- **Files modified:** 4

## Accomplishments

- Created Info.plist with LSUIElement=true for Dock-less menu bar app
- Built Terminal Remote.app bundle with build-bundle.sh script
- Implemented login item toggle using SMAppService (macOS 13+)
- Fixed critical tray icon rendering with winit event loop

## Task Commits

Each task was committed atomically:

1. **Task 1: Create Info.plist** - `14b9d65` (feat)
2. **Task 2: Create app bundle build script** - `e4568f5` (feat)
3. **Task 3: Implement login item toggle** - `cfe4818` (feat)
4. **Task 4: Human verification + winit fix** - `5f180e4` (fix)

## Files Created/Modified

- `mac-client/Info.plist` - App bundle configuration with LSUIElement, bundle IDs, minimum OS version
- `mac-client/build-bundle.sh` - Shell script to create .app bundle from release binary
- `mac-client/Cargo.toml` - Added smappservice-rs and winit dependencies
- `mac-client/src/main.rs` - Rewrote with winit EventLoop, ApplicationHandler trait, login item functions

## Decisions Made

1. **winit event loop required** - macOS needs a proper run loop for tray icons to appear in the menu bar. The original polling loop approach didn't trigger AppKit's display mechanism.

2. **ApplicationHandler trait pattern** - Winit 0.30's `run_app()` requires implementing `ApplicationHandler<AppEvent>` for the main application struct. This provides proper lifecycle management.

3. **Event handler bridging** - `TrayIconEvent::set_event_handler` and `MenuEvent::set_event_handler` forward events from the tray-icon/muda crates to our winit event loop via `EventLoopProxy::send_event`.

4. **LSMinimumSystemVersion 13.0** - SMAppService (login items API) requires macOS 13 Ventura or later. Setting this in Info.plist ensures compatibility.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Tray icon not appearing without proper event loop**
- **Found during:** Task 4 (Human verification)
- **Issue:** Tray icon was being created but not rendering in menu bar
- **Fix:** Rewrote main.rs to use winit's EventLoop with ApplicationHandler trait
- **Files modified:** mac-client/Cargo.toml (added winit 0.30), mac-client/src/main.rs
- **Verification:** Tray icon now appears and responds to clicks
- **Committed in:** 5f180e4

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Critical fix required for basic functionality. The winit event loop is the correct pattern for macOS menu bar applications.

## Issues Encountered

- Original polling loop (thread::sleep + try_recv) didn't work for tray icon rendering on macOS. AppKit requires a proper run loop that pumps events correctly. Winit provides this through its EventLoop.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 5 (Mac Client) is now complete with all 13 CLIENT requirements:
- CLIENT-01: App runs as menu bar app with no Dock icon
- CLIENT-02: WebSocket connection with auto-reconnect
- CLIENT-03: Unix socket listener at /tmp/terminal-remote.sock
- CLIENT-04: Session tracking infrastructure
- CLIENT-05: Status icon visible in menu bar
- CLIENT-06: Click icon opens dropdown menu
- CLIENT-07: Session code displayed in menu
- CLIENT-08: Copy session code to clipboard works
- CLIENT-09: Connection status indicator works
- CLIENT-10: Quit option works
- CLIENT-11: Start at login option available
- CLIENT-12: Template image adapts to dark/light mode
- CLIENT-13: Session count displayed in menu

Ready for Phase 6: Shell Integration - The Unix socket IPC server is running and ready for shell scripts to connect.

---
*Phase: 05-mac-client*
*Plan: 05*
*Completed: 2026-02-06*
