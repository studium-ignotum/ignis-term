/**
 * Session manager for routing terminal I/O between iTerm2 bridge and relay.
 *
 * Sits between the ITerm2Bridge (Python subprocess IPC) and the
 * ConnectionManager (WebSocket to relay). Translates between the Python
 * bridge's IPC format and the WebSocket protocol format defined in
 * src/shared/protocol.ts.
 *
 * Data flow:
 *   iTerm2 -> Python bridge -> Unix socket -> ITerm2Bridge -> SessionManager -> relay -> browser
 *   browser -> relay -> ConnectionManager -> SessionManager -> ITerm2Bridge -> Unix socket -> Python bridge -> iTerm2
 */

import { ITerm2Bridge, type SessionInfo } from "./iterm-bridge.js";

interface TabInfo {
  tabId: string;
  sessionId: string;
  title: string;
  isActive: boolean;
}

export class SessionManager {
  private bridge: ITerm2Bridge;
  private sendToRelay: (message: string) => void;
  private tabs: TabInfo[] = [];

  constructor(sendToRelay: (message: string) => void) {
    this.sendToRelay = sendToRelay;
    this.bridge = new ITerm2Bridge();
    this.setupBridgeHandlers();
  }

  /**
   * Wire up event handlers on the iTerm2 bridge to translate
   * IPC messages into WebSocket protocol messages for the relay.
   */
  private setupBridgeHandlers(): void {
    // Terminal output from iTerm2 -> relay -> browser
    // Third param is optional message type (for distinguishing initial_terminal_data)
    this.bridge.on(
      "terminal_data",
      (sessionId: string, data: Buffer, msgType?: string) => {
        const type =
          msgType === "initial_terminal_data"
            ? "initial_terminal_data"
            : "terminal_data";
        // console.log(`[SessionManager] Sending ${type} for session ${sessionId}, ${data.length} bytes`);
        this.sendToRelay(
          JSON.stringify({
            type,
            sessionId,
            payload: data.toString("utf-8"),
          }),
        );
      },
    );

    // Session list -> tab_list message
    this.bridge.on("sessions", (sessions: SessionInfo[]) => {
      console.log(
        `[SessionManager] Received ${sessions.length} sessions from bridge`,
      );
      this.tabs = sessions.map((s) => ({
        tabId: s.tab_id,
        sessionId: s.session_id,
        title: s.title || "Shell",
        isActive: s.is_active,
      }));
      this.sendToRelay(
        JSON.stringify({
          type: "tab_list",
          tabs: this.tabs,
        }),
      );
    });

    // Tab focus changed in iTerm2 -> notify browser
    this.bridge.on("tab_switched", (tabId: string, sessionId?: string) => {
      this.sendToRelay(
        JSON.stringify({
          type: "tab_switch",
          tabId,
          sessionId,
        }),
      );
    });

    // Tabs changed (layout) -> re-send full list
    this.bridge.on("tabs_changed", (tabs: SessionInfo[]) => {
      this.tabs = tabs.map((s) => ({
        tabId: s.tab_id,
        sessionId: s.session_id,
        title: s.title || "Shell",
        isActive: s.is_active,
      }));
      this.sendToRelay(
        JSON.stringify({
          type: "tab_list",
          tabs: this.tabs,
        }),
      );
    });

    // iTerm2 config -> config message for browser's xterm.js setup
    this.bridge.on("config", (config: Record<string, unknown>) => {
      // Translate Python bridge config format to protocol ConfigMessage format
      const cursorStyleMap: Record<string, string> = {
        CURSOR_TYPE_BLOCK: "block",
        CURSOR_TYPE_UNDERLINE: "underline",
        CURSOR_TYPE_VERTICAL: "bar",
      };

      const theme: Record<string, string> = {
        foreground: (config.foreground as string) || "#ffffff",
        background: (config.background as string) || "#000000",
        cursor: (config.cursor as string) || "#ffffff",
        selectionBackground: (config.selectionColor as string) || "#555555",
      };

      // Map ANSI colors to theme keys
      const ansiNames = [
        "black",
        "red",
        "green",
        "yellow",
        "blue",
        "magenta",
        "cyan",
        "white",
        "brightBlack",
        "brightRed",
        "brightGreen",
        "brightYellow",
        "brightBlue",
        "brightMagenta",
        "brightCyan",
        "brightWhite",
      ];

      const ansiColors = config.ansiColors as string[] | undefined;
      if (ansiColors) {
        ansiColors.forEach((color: string, i: number) => {
          if (ansiNames[i]) {
            theme[ansiNames[i]] = color;
          }
        });
      }

      // Parse font name and size from iTerm2 format "FontName Size"
      const fontStr = (config.font as string) || "";
      const fontParts = fontStr.split(" ");
      const fontSize = parseInt(fontParts.pop() || "13", 10);
      const fontName = fontParts.join(" ") || "Menlo";

      const cursorType = (config.cursorType as string) || "";

      this.sendToRelay(
        JSON.stringify({
          type: "config",
          font: fontName,
          fontSize: isNaN(fontSize) ? 13 : fontSize,
          cursorStyle: cursorStyleMap[cursorType] || "block",
          cursorBlink: Boolean(config.cursorBlink ?? true),
          scrollback: (config.scrollback as number) || 10000,
          theme,
        }),
      );
    });

    this.bridge.on("ready", () => {
      console.log("[SessionManager] iTerm2 bridge ready");
    });

    this.bridge.on("error", (err: Error) => {
      console.error("[SessionManager] Bridge error:", err.message);
    });
  }

  /**
   * Handle a message received from the relay (originated from browser).
   * Parses the message and forwards the appropriate command to the bridge.
   */
  handleRelayMessage(raw: string): void {
    try {
      const msg = JSON.parse(raw);
      switch (msg.type) {
        case "terminal_input":
          this.bridge.sendInput(msg.sessionId, msg.payload);
          break;
        case "terminal_resize":
          this.bridge.sendResize(msg.sessionId, msg.cols, msg.rows);
          break;
        case "tab_switch":
          console.log(
            `[SessionManager] Received tab_switch for tabId: ${msg.tabId}`,
          );
          this.bridge.switchTab(msg.tabId);
          break;
        case "tab_create":
          this.bridge.createTab();
          break;
        case "tab_close":
          this.bridge.closeTab(msg.tabId);
          break;
        case "request_screen_refresh":
          this.bridge.requestScreenRefresh(msg.sessionId);
          break;
        case "browser_connected":
          // Browser (re)connected - resend all initial state
          console.log(
            "[SessionManager] Browser connected/reconnected, requesting full state resend",
          );
          this.bridge.resendInitialState();
          break;
        default:
          // Ignore other message types (ping, pong, registered, etc.)
          break;
      }
    } catch (err) {
      console.error("[SessionManager] Failed to handle relay message:", err);
    }
  }

  /**
   * Start the iTerm2 bridge subprocess.
   */
  async start(): Promise<void> {
    await this.bridge.start();
  }

  /**
   * Stop the iTerm2 bridge subprocess and clean up.
   */
  async stop(): Promise<void> {
    await this.bridge.stop();
  }
}
