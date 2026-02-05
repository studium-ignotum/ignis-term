#!/usr/bin/env python3
"""
iTerm2 Python API bridge for remote terminal access.

Runs as a subprocess of the Node.js Mac client. Uses the iTerm2 Python API
to discover sessions, attach coprocesses for raw PTY capture, monitor tab
changes, and read profile configuration.

Communication: JSON lines over Unix domain socket.
Each line is a JSON object terminated by newline.

Usage: python3 iterm-bridge.py [socket_path]
"""

import iterm2
import asyncio
import json
import os
import sys
import signal
import base64
import atexit
import logging
import subprocess

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [iterm-bridge] %(levelname)s: %(message)s",
)
log = logging.getLogger(__name__)

SOCKET_PATH = sys.argv[1] if len(sys.argv) > 1 else "/tmp/iterm-bridge.sock"
COPROCESS_SCRIPT = os.path.join(
    os.path.dirname(os.path.abspath(__file__)), "coprocess-bridge.sh"
)

# Track all socket paths for cleanup
_socket_paths_to_clean = set()


def _cleanup_sockets():
    """Remove all socket files on exit."""
    for path in _socket_paths_to_clean:
        try:
            os.unlink(path)
        except OSError:
            pass


atexit.register(_cleanup_sockets)


class ITerm2Bridge:
    """
    Bridge between iTerm2 Python API and Node.js Mac client.

    Discovers all existing iTerm2 sessions on startup, attaches coprocesses
    for raw PTY byte capture, monitors tab focus/layout changes in real-time,
    reads iTerm2 profile configuration, and communicates with the Node.js Mac
    client via Unix domain socket using JSON lines protocol.

    Message types FROM Python -> Node.js:
      - {"type": "sessions", "sessions": [...]}
      - {"type": "terminal_data", "session_id": "...", "data": "base64..."}
      - {"type": "tab_switched", "tab_id": "..."}
      - {"type": "config", ...}
      - {"type": "ready"}
      - {"type": "error", "message": "..."}

    Message types FROM Node.js -> Python:
      - {"type": "terminal_input", "session_id": "...", "data": "base64..."}
      - {"type": "terminal_resize", "session_id": "...", "cols": N, "rows": N}
      - {"type": "tab_switch", "tab_id": "..."}
      - {"type": "tab_create"}
      - {"type": "tab_close", "tab_id": "..."}
    """

    def __init__(self):
        self.connection = None
        self.app = None
        self.client_writer = None
        self.client_connected = asyncio.Event()
        self.session_map = {}  # session_id -> session object
        self.coprocess_sockets = {}  # session_id -> (reader, writer)
        self.coprocess_servers = {}  # session_id -> server object
        self._running = True
        self._initial_ready_sent = False

    async def main(self, connection):
        """Entry point called by iterm2.run_until_complete."""
        self.connection = connection
        self.app = await iterm2.async_get_app(connection)

        if self.app is None:
            log.error("Failed to get iTerm2 app — is iTerm2 running?")
            sys.exit(1)

        log.info("Connected to iTerm2, starting socket server on %s", SOCKET_PATH)

        # Clean up stale socket file
        try:
            os.unlink(SOCKET_PATH)
        except OSError:
            pass

        _socket_paths_to_clean.add(SOCKET_PATH)

        server = await asyncio.start_unix_server(
            self._handle_client, SOCKET_PATH
        )

        log.info("Socket server listening, waiting for Node.js client...")

        try:
            await server.serve_forever()
        except asyncio.CancelledError:
            pass
        finally:
            server.close()
            await server.wait_closed()

    async def _handle_client(self, reader, writer):
        """Handle a Node.js client connection."""
        log.info("Node.js client connected")
        self.client_writer = writer
        self.client_connected.set()

        try:
            # Discover all existing sessions and start coprocesses
            await self._enumerate_and_send_sessions()

            # Send iTerm2 profile configuration
            await self._send_config()

            # Signal that initial setup is complete
            await self._send_to_client({"type": "ready"})
            self._initial_ready_sent = True

            # Run all monitors concurrently
            await asyncio.gather(
                self._monitor_focus(),
                self._monitor_layout(),
                self._monitor_new_sessions(),
                self._read_client_commands(reader),
            )
        except asyncio.CancelledError:
            log.info("Client handler cancelled")
        except ConnectionResetError:
            log.warning("Node.js client disconnected")
        except Exception as exc:
            log.error("Client handler error: %s", exc, exc_info=True)
            try:
                await self._send_to_client({
                    "type": "error",
                    "message": str(exc),
                })
            except Exception:
                pass
        finally:
            self.client_writer = None
            self.client_connected.clear()
            writer.close()
            try:
                await writer.wait_closed()
            except Exception:
                pass
            log.info("Client connection closed")

    # ──────────────────────────────────────────────────────────────
    # Session enumeration
    # ──────────────────────────────────────────────────────────────

    async def _enumerate_and_send_sessions(self):
        """Discover all iTerm2 sessions, start coprocesses, send list to client."""
        self.app = await iterm2.async_get_app(self.connection)
        sessions_info = []

        # Determine the currently active session for is_active flag
        active_session_id = None
        if (
            self.app.current_terminal_window
            and self.app.current_terminal_window.current_tab
            and self.app.current_terminal_window.current_tab.current_session
        ):
            active_session_id = (
                self.app.current_terminal_window.current_tab.current_session.session_id
            )

        for window in self.app.terminal_windows:
            for tab in window.tabs:
                for session in tab.sessions:
                    session_id = session.session_id
                    self.session_map[session_id] = session

                    sessions_info.append({
                        "session_id": session_id,
                        "tab_id": tab.tab_id,
                        "title": session.name or "",
                        "is_active": session_id == active_session_id,
                    })

                    # Start coprocess if not already running for this session
                    if session_id not in self.coprocess_sockets:
                        await self._start_coprocess(session)

        await self._send_to_client({
            "type": "sessions",
            "sessions": sessions_info,
        })

        log.info(
            "Enumerated %d sessions across %d windows",
            len(sessions_info),
            len(self.app.terminal_windows),
        )

        # Send initial screen contents for the active session so browser shows current state
        if active_session_id:
            await self._send_session_screen(active_session_id)

    # ──────────────────────────────────────────────────────────────
    # Coprocess management
    # ──────────────────────────────────────────────────────────────

    async def _start_coprocess(self, session):
        """Start coprocess-bridge.sh for an iTerm2 session."""
        session_id = session.session_id
        # Sanitize session_id for use in file path
        safe_id = session_id.replace(":", "-").replace("/", "-")
        data_socket_path = f"/tmp/iterm-coprocess-{safe_id}.sock"

        # Clean up stale socket file
        try:
            os.unlink(data_socket_path)
        except OSError:
            pass

        _socket_paths_to_clean.add(data_socket_path)

        # Start Unix socket server for this coprocess's data BEFORE launching
        # the coprocess (so the socket is ready when it connects)
        coprocess_server = await asyncio.start_unix_server(
            lambda r, w, sid=session_id: self._handle_coprocess_data(sid, r, w),
            data_socket_path,
        )
        self.coprocess_servers[session_id] = coprocess_server

        # Launch the coprocess via iTerm2 API
        cmd = f"{COPROCESS_SCRIPT} {session_id} {data_socket_path}"
        try:
            success = await session.async_run_coprocess(cmd)
            if not success:
                log.warning(
                    "Coprocess already running for session %s. Attempting to kill old instance...", 
                    session_id
                )
                
                # Attempt to kill old coprocess bridge script for this session
                # Pattern matches command line arguments: coprocess-bridge.sh <session_id>
                try:
                    subprocess.run(["pkill", "-f", f"coprocess-bridge.sh {session_id}"], check=False)
                    await asyncio.sleep(0.5) # Give it time to die and iTerm2 to handle exit
                    
                    # Retry starting the coprocess
                    success = await session.async_run_coprocess(cmd)
                except Exception as e:
                    log.error("Failed to kill/retry coprocess: %s", e)

            if not success:
                log.warning(
                    "Coprocess start finally failed for session %s",
                    session_id,
                )
                # Clean up the server we just created
                coprocess_server.close()
                await coprocess_server.wait_closed()
                del self.coprocess_servers[session_id]
                try:
                    os.unlink(data_socket_path)
                except OSError:
                    pass
                _socket_paths_to_clean.discard(data_socket_path)
            else:
                log.info("Started coprocess for session %s", session_id)
        except Exception as exc:
            log.error(
                "Failed to start coprocess for session %s: %s",
                session_id,
                exc,
            )
            coprocess_server.close()
            await coprocess_server.wait_closed()
            del self.coprocess_servers[session_id]
            try:
                os.unlink(data_socket_path)
            except OSError:
                pass
            _socket_paths_to_clean.discard(data_socket_path)

    async def _handle_coprocess_data(self, session_id, reader, writer):
        """
        Receive raw PTY output from a coprocess, forward to Node.js client
        as base64-encoded terminal data.
        """
        log.info("Coprocess connected for session %s", session_id)
        self.coprocess_sockets[session_id] = (reader, writer)

        try:
            while True:
                data = await reader.read(65536)
                if not data:
                    break

                # Forward to Node.js client as base64-encoded bytes
                await self._send_to_client({
                    "type": "terminal_data",
                    "session_id": session_id,
                    "data": base64.b64encode(data).decode("ascii"),
                })
        except asyncio.CancelledError:
            pass
        except ConnectionResetError:
            log.info("Coprocess disconnected for session %s", session_id)
        except Exception as exc:
            log.error(
                "Coprocess data error for session %s: %s",
                session_id,
                exc,
            )
        finally:
            self.coprocess_sockets.pop(session_id, None)
            writer.close()
            try:
                await writer.wait_closed()
            except Exception:
                pass
            log.info("Coprocess data handler ended for session %s", session_id)

    async def _stop_coprocess(self, session_id):
        """Stop coprocess resources for a session."""
        # Close the data socket connection
        if session_id in self.coprocess_sockets:
            _, writer = self.coprocess_sockets.pop(session_id)
            writer.close()
            try:
                await writer.wait_closed()
            except Exception:
                pass

        # Close the server
        if session_id in self.coprocess_servers:
            server = self.coprocess_servers.pop(session_id)
            server.close()
            await server.wait_closed()

        # Clean up socket file
        safe_id = session_id.replace(":", "-").replace("/", "-")
        data_socket_path = f"/tmp/iterm-coprocess-{safe_id}.sock"
        try:
            os.unlink(data_socket_path)
        except OSError:
            pass
        _socket_paths_to_clean.discard(data_socket_path)

    # ──────────────────────────────────────────────────────────────
    # Monitors (focus, layout, new sessions)
    # ──────────────────────────────────────────────────────────────

    async def _monitor_focus(self):
        """Watch for tab focus changes using iTerm2 FocusMonitor."""
        try:
            async with iterm2.FocusMonitor(self.connection) as monitor:
                while True:
                    update = await monitor.async_get_next_update()
                    if update.selected_tab_changed:
                        tab_id = update.selected_tab_changed.tab_id
                        log.info("Tab focus changed: %s", tab_id)
                        
                        # Find the active session within the switched tab
                        active_session_id = None
                        self.app = await iterm2.async_get_app(self.connection)
                        if (
                            self.app.current_terminal_window
                            and self.app.current_terminal_window.current_tab
                            and self.app.current_terminal_window.current_tab.current_session
                        ):
                            active_session_id = (
                                self.app.current_terminal_window.current_tab.current_session.session_id
                            )
                            log.info("Active session in tab: %s", active_session_id)
                        
                        await self._send_to_client({
                            "type": "tab_switched",
                            "tab_id": tab_id,
                            "session_id": active_session_id,
                        })

                        # Automatically send current screen contents for the new session
                        if active_session_id:
                            await self._send_session_screen(active_session_id)
        except asyncio.CancelledError:
            pass
        except Exception as exc:
            log.error("Focus monitor error: %s", exc, exc_info=True)

    async def _send_session_screen(self, session_id):
        """Fetch and send the current screen contents for a session with colors if possible."""
        try:
            session = self.session_map.get(session_id)
            if not session:
                # Try to find the session directly via iTerm2 API
                log.warning("Session %s not in session_map, trying iTerm2 API lookup", session_id)
                self.app = await iterm2.async_get_app(self.connection)
                session = self.app.get_session_by_id(session_id)
                if session:
                    # Add to map for future use
                    self.session_map[session_id] = session
                else:
                    log.error("Session %s not found anywhere, available: %s",
                              session_id, list(self.session_map.keys()))
                    return

            # Try to get styled content with a short timeout
            contents = None
            try:
                async with session.get_screen_streamer(want_contents=True) as streamer:
                    # Use a short timeout - if screen was recently updated, we get styled content
                    contents = await asyncio.wait_for(streamer.async_get(style=True), timeout=0.1)
                    if contents:
                        log.info("Got styled screen contents")
            except asyncio.TimeoutError:
                log.debug("ScreenStreamer timed out, falling back to plain text")
            except Exception as e:
                log.debug("ScreenStreamer failed: %s, falling back to plain text", e)

            # Fallback to plain text if styled content not available
            if not contents:
                contents = await session.async_get_screen_contents()
                if not contents:
                    return

            log.info("Screen contents lines: %d", contents.number_of_lines)

            # Build content - try styled rendering first
            output_lines = []
            has_style = False
            for i in range(contents.number_of_lines):
                line = contents.line(i)
                # Check if this line has style information
                if hasattr(line, 'style_at') and line.style_at(0) is not None:
                    has_style = True
                    line_output = self._render_line_with_style(line)
                else:
                    line_output = line.string.rstrip()
                output_lines.append(line_output)

            # Build final content
            clear_screen = "\x1b[2J"
            cursor_home = "\x1b[H"
            content = "\r\n".join(output_lines)
            reset = "\x1b[0m" if has_style else ""
            cursor_pos = f"\x1b[{contents.number_of_lines};1H"

            full_text = clear_screen + cursor_home + content + reset + cursor_pos

            # Send as initial_terminal_data
            await self._send_to_client({
                "type": "initial_terminal_data",
                "session_id": session_id,
                "data": base64.b64encode(full_text.encode("utf-8")).decode("ascii"),
            })
            log.info("Sent initial screen for session %s (%d lines, styled=%s)",
                     session_id, len(output_lines), has_style)

        except Exception as exc:
            log.error("Failed to send screen content for %s: %s", session_id, exc, exc_info=True)

    def _render_line_with_style(self, line):
        """Render a line with ANSI color codes based on cell styles."""
        result = []
        line_str = line.string
        prev_fg = None
        prev_bg = None
        prev_attrs = None

        for x, char in enumerate(line_str):
            style = line.style_at(x)
            if style:
                # Get current style attributes
                curr_attrs = (style.bold, style.italic, style.underline,
                              style.faint, style.inverse, style.strikethrough)
                curr_fg = self._get_color_tuple(style.fg_color)
                curr_bg = self._get_color_tuple(style.bg_color)

                # Emit ANSI codes if style changed
                if curr_attrs != prev_attrs or curr_fg != prev_fg or curr_bg != prev_bg:
                    codes = self._build_ansi_codes(style, curr_fg, curr_bg)
                    if codes:
                        result.append(codes)
                    prev_attrs = curr_attrs
                    prev_fg = curr_fg
                    prev_bg = curr_bg

            result.append(char)

        return "".join(result).rstrip()

    def _get_color_tuple(self, color):
        """Get a tuple representation of a color for comparison."""
        if color is None:
            return None
        try:
            if color.is_rgb:
                rgb = color.rgb
                return ("rgb", int(rgb.red), int(rgb.green), int(rgb.blue))
            if color.is_standard:
                return ("std", color.standard)
        except Exception:
            pass
        return None

    def _build_ansi_codes(self, style, fg_tuple, bg_tuple):
        """Build ANSI escape codes for the given style."""
        codes = []

        # Reset first to clear previous state
        codes.append("0")

        # Text attributes
        if style.bold:
            codes.append("1")
        if style.faint:
            codes.append("2")
        if style.italic:
            codes.append("3")
        if style.underline:
            codes.append("4")
        if style.inverse:
            codes.append("7")
        if style.strikethrough:
            codes.append("9")

        # Foreground color
        if fg_tuple:
            if fg_tuple[0] == "rgb":
                codes.append(f"38;2;{fg_tuple[1]};{fg_tuple[2]};{fg_tuple[3]}")
            elif fg_tuple[0] == "std":
                codes.append(f"38;5;{fg_tuple[1]}")

        # Background color
        if bg_tuple:
            if bg_tuple[0] == "rgb":
                codes.append(f"48;2;{bg_tuple[1]};{bg_tuple[2]};{bg_tuple[3]}")
            elif bg_tuple[0] == "std":
                codes.append(f"48;5;{bg_tuple[1]}")

        if codes:
            return f"\x1b[{';'.join(codes)}m"
        return ""

    async def _monitor_layout(self):
        """Watch for tab creation/deletion using iTerm2 LayoutChangeMonitor."""
        try:
            async with iterm2.LayoutChangeMonitor(self.connection) as monitor:
                while True:
                    await monitor.async_get()
                    log.info("Layout changed, re-enumerating sessions")
                    # Refresh the app state and re-send session list
                    await self._enumerate_and_send_sessions()
        except asyncio.CancelledError:
            pass
        except Exception as exc:
            log.error("Layout monitor error: %s", exc, exc_info=True)

    async def _monitor_new_sessions(self):
        """Watch for new sessions and automatically attach coprocesses."""
        try:
            async with iterm2.NewSessionMonitor(self.connection) as monitor:
                while True:
                    session_id = await monitor.async_get()
                    log.info("New session detected: %s", session_id)
                    # Refresh app to get the new session object
                    self.app = await iterm2.async_get_app(self.connection)
                    session = self.app.get_session_by_id(session_id)
                    if session:
                        self.session_map[session_id] = session
                        await self._start_coprocess(session)
        except asyncio.CancelledError:
            pass
        except Exception as exc:
            log.error("New session monitor error: %s", exc, exc_info=True)

    # ──────────────────────────────────────────────────────────────
    # Client command handling
    # ──────────────────────────────────────────────────────────────

    async def _read_client_commands(self, reader):
        """Read JSON line commands from the Node.js client."""
        buffer = b""
        try:
            while True:
                data = await reader.read(65536)
                if not data:
                    log.info("Client reader got EOF")
                    break
                buffer += data
                while b"\n" in buffer:
                    line, buffer = buffer.split(b"\n", 1)
                    if not line.strip():
                        continue
                    try:
                        msg = json.loads(line)
                        await self._handle_command(msg)
                    except json.JSONDecodeError as exc:
                        log.warning("Invalid JSON from client: %s", exc)
        except asyncio.CancelledError:
            pass
        except ConnectionResetError:
            log.info("Client disconnected during read")

    async def _handle_command(self, msg):
        """Dispatch a command from the Node.js client."""
        cmd_type = msg.get("type")

        if cmd_type == "terminal_input":
            await self._handle_terminal_input(msg)
        elif cmd_type == "terminal_resize":
            await self._handle_terminal_resize(msg)
        elif cmd_type == "tab_switch":
            await self._switch_tab(msg["tab_id"])
        elif cmd_type == "tab_create":
            await self._create_tab()
        elif cmd_type == "tab_close":
            await self._close_tab(msg["tab_id"])
        elif cmd_type == "request_screen_refresh":
            await self._handle_screen_refresh_request(msg)
        elif cmd_type == "resend_initial_state":
            # Browser (re)connected - resend sessions, config, and screen content
            log.info("Browser reconnected, resending initial state")
            await self._resend_initial_state()
        else:
            log.warning("Unknown command type: %s", cmd_type)

    async def _handle_screen_refresh_request(self, msg):
        """Handle request from browser to refresh screen content for a session."""
        session_id = msg.get("session_id")
        if session_id:
            log.info("Screen refresh requested for session: %s", session_id)
            await self._send_session_screen(session_id)
        else:
            # If no session specified, refresh the active session
            if (
                self.app
                and self.app.current_terminal_window
                and self.app.current_terminal_window.current_tab
                and self.app.current_terminal_window.current_tab.current_session
            ):
                active_session_id = (
                    self.app.current_terminal_window.current_tab.current_session.session_id
                )
                log.info("Screen refresh requested for active session: %s", active_session_id)
                await self._send_session_screen(active_session_id)

    async def _resend_initial_state(self):
        """Resend all initial state when browser reconnects."""
        try:
            # Re-enumerate sessions (also sends initial screen content for active session)
            await self._enumerate_and_send_sessions()
            # Resend config
            await self._send_config()
            # Send ready signal
            await self._send_to_client({"type": "ready"})
            log.info("Resent initial state to reconnected browser")
        except Exception as exc:
            log.error("Failed to resend initial state: %s", exc, exc_info=True)

    async def _handle_terminal_input(self, msg):
        """Forward keyboard input to a coprocess (becomes iTerm2 keyboard input)."""
        session_id = msg.get("session_id")
        raw_data = msg.get("data", "")

        if not session_id:
            return

        data = base64.b64decode(raw_data)

        if session_id in self.coprocess_sockets:
            _, writer = self.coprocess_sockets[session_id]
            try:
                writer.write(data)
                await writer.drain()
            except (ConnectionResetError, BrokenPipeError):
                log.warning(
                    "Coprocess write failed for session %s (disconnected)",
                    session_id,
                )
                self.coprocess_sockets.pop(session_id, None)
        else:
            # Fallback: use iTerm2 API to send text directly
            session = self.session_map.get(session_id)
            if session:
                try:
                    await session.async_send_text(data.decode("utf-8", errors="replace"))
                except Exception as exc:
                    log.warning("Failed to send text to session %s: %s", session_id, exc)

    async def _handle_terminal_resize(self, msg):
        """Handle terminal resize request.

        iTerm2 manages PTY size through the session. When the browser terminal
        resizes, we can use the iTerm2 API to adjust the session size, or let
        the coprocess/PTY handle it automatically if the session is displayed.
        """
        session_id = msg.get("session_id")
        cols = msg.get("cols")
        rows = msg.get("rows")

        if not session_id or not cols or not rows:
            return

        # iTerm2 controls the PTY size based on the session's display area.
        # Remote resize is informational -- the actual PTY resize happens
        # through iTerm2's window/split management. Log for debugging.
        log.debug(
            "Resize request for session %s: %dx%d (informational)",
            session_id,
            cols,
            rows,
        )

    async def _switch_tab(self, tab_id):
        """Switch to a tab by ID in iTerm2 and send screen content to browser."""
        self.app = await iterm2.async_get_app(self.connection)

        # Try to find by tab_id first
        for window in self.app.terminal_windows:
            for tab in window.tabs:
                if tab.tab_id == tab_id:
                    await tab.async_select()
                    log.info("Switched to tab %s", tab_id)
                    # Send screen content for the active session in this tab
                    if tab.current_session:
                        await self._send_session_screen(tab.current_session.session_id)
                    return

        # Fallback: Check if tab_id is actually a session_id
        # The browser sending session IDs as tab IDs is a known behavior
        session = self.app.get_session_by_id(tab_id)
        if session:
            await session.async_activate()
            log.info("Activated session %s (via switch_tab)", tab_id)
            # Send screen content for this session
            await self._send_session_screen(tab_id)
            return

        # Log available tabs for debugging
        all_tabs = []
        for w in self.app.terminal_windows:
            for t in w.tabs:
                all_tabs.append(t.tab_id)
        log.warning("Tab/Session %s not found. Available Tabs: %s", tab_id, all_tabs)

    async def _create_tab(self):
        """Create a new tab in the current iTerm2 window."""
        self.app = await iterm2.async_get_app(self.connection)
        window = self.app.current_terminal_window
        if window:
            tab = await window.async_create_tab()
            log.info("Created new tab %s", tab.tab_id)
        else:
            log.warning("No current window to create tab in")

    async def _close_tab(self, tab_id):
        """Close a tab by ID in iTerm2."""
        self.app = await iterm2.async_get_app(self.connection)
        for window in self.app.terminal_windows:
            for tab in window.tabs:
                if tab.tab_id == tab_id:
                    # Stop coprocess for all sessions in this tab
                    for session in tab.sessions:
                        await self._stop_coprocess(session.session_id)
                        self.session_map.pop(session.session_id, None)
                    await tab.async_close()
                    log.info("Closed tab %s", tab_id)
                    return
        log.warning("Tab %s not found for close", tab_id)

    # ──────────────────────────────────────────────────────────────
    # Configuration reading
    # ──────────────────────────────────────────────────────────────

    async def _send_config(self):
        """Read iTerm2 profile configuration and send to client."""
        try:
            window = self.app.current_terminal_window
            if not window or not window.current_tab or not window.current_tab.current_session:
                log.warning("No active session to read config from")
                return

            session = window.current_tab.current_session
            profile = await session.async_get_profile()

            # Determine scrollback lines
            scrollback = 100000  # default for unlimited
            try:
                if not profile.unlimited_scrollback:
                    scrollback = profile.scrollback_lines
            except Exception:
                pass

            # Build config message
            config = {
                "type": "config",
                "font": self._safe_get(profile, "normal_font", "Monaco 12"),
                "cursorType": str(self._safe_get(profile, "cursor_type", "CURSOR_TYPE_BLOCK")),
                "cursorBlink": self._safe_get(profile, "blinking_cursor", False),
                "scrollback": scrollback,
                "foreground": self._color_to_hex(
                    self._safe_get(profile, "foreground_color", None)
                ),
                "background": self._color_to_hex(
                    self._safe_get(profile, "background_color", None)
                ),
                "cursor": self._color_to_hex(
                    self._safe_get(profile, "cursor_color", None)
                ),
                "selectionColor": self._color_to_hex(
                    self._safe_get(profile, "selection_color", None)
                ),
                "ansiColors": [
                    self._color_to_hex(
                        self._safe_get(profile, f"ansi_{i}_color", None)
                    )
                    for i in range(16)
                ],
            }

            await self._send_to_client(config)
            log.info("Sent iTerm2 profile config to client")

        except Exception as exc:
            log.error("Failed to read iTerm2 config: %s", exc, exc_info=True)
            await self._send_to_client({
                "type": "error",
                "message": f"Failed to read config: {exc}",
            })

    @staticmethod
    def _safe_get(obj, attr, default=None):
        """Safely get an attribute, returning default on error."""
        try:
            return getattr(obj, attr)
        except Exception:
            return default

    @staticmethod
    def _color_to_hex(color):
        """Convert an iTerm2 Color object to hex string (#rrggbb)."""
        if color is None:
            return "#ffffff"
        try:
            r = int(color.red * 255)
            g = int(color.green * 255)
            b = int(color.blue * 255)
            return f"#{r:02x}{g:02x}{b:02x}"
        except Exception:
            return "#ffffff"

    # ──────────────────────────────────────────────────────────────
    # IPC helpers
    # ──────────────────────────────────────────────────────────────

    async def _send_to_client(self, msg):
        """Send a JSON message to the Node.js client (JSON lines protocol)."""
        if self.client_writer is None:
            return
        try:
            line = json.dumps(msg) + "\n"
            self.client_writer.write(line.encode("utf-8"))
            await self.client_writer.drain()
        except (ConnectionResetError, BrokenPipeError):
            log.warning("Failed to send to client (disconnected)")
            self.client_writer = None
            self.client_connected.clear()


def main():
    """Launch the iTerm2 bridge."""
    log.info("Starting iTerm2 bridge, socket: %s", SOCKET_PATH)
    log.info("Coprocess script: %s", COPROCESS_SCRIPT)

    if not os.path.isfile(COPROCESS_SCRIPT):
        log.error("Coprocess script not found: %s", COPROCESS_SCRIPT)
        sys.exit(1)

    if not os.access(COPROCESS_SCRIPT, os.X_OK):
        log.error("Coprocess script is not executable: %s", COPROCESS_SCRIPT)
        sys.exit(1)

    bridge = ITerm2Bridge()

    try:
        iterm2.run_until_complete(bridge.main)
    except Exception as exc:
        log.error("iTerm2 bridge failed: %s", exc, exc_info=True)
        log.error(
            "Ensure iTerm2 is running and the Python API is enabled "
            "(Preferences > General > Magic > Enable Python API)"
        )
        sys.exit(1)


if __name__ == "__main__":
    main()
