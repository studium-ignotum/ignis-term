/**
 * iTerm2 Python bridge subprocess and Unix socket IPC manager.
 *
 * Manages the lifecycle of the Python bridge subprocess (iterm-bridge.py),
 * connects to it via Unix domain socket, and provides a typed event-based
 * interface for terminal data, session info, tab management, and config.
 *
 * Communication: JSON lines over Unix domain socket.
 * Auto-restarts the Python subprocess on crash.
 */

import { spawn, type ChildProcess } from "child_process";
import { createConnection, type Socket } from "net";
import { EventEmitter } from "events";
import { fileURLToPath } from "url";
import * as path from "path";
import * as fs from "fs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const SOCKET_PATH = "/tmp/iterm-bridge.sock";
const PYTHON_SCRIPT = path.join(__dirname, "..", "iterm-bridge.py");
const VENV_PYTHON = path.join(__dirname, "..", ".venv", "bin", "python3");
const RESTART_DELAY_MS = 3000;
const MAX_CONNECT_RETRIES = 10;
const CONNECT_RETRY_DELAY_MS = 500;

export interface SessionInfo {
  session_id: string;
  tab_id: string;
  title: string;
  is_active: boolean;
}

export interface BridgeEvents {
  terminal_data: (sessionId: string, data: Buffer, msgType?: string) => void;
  sessions: (sessions: SessionInfo[]) => void;
  tab_switched: (tabId: string, sessionId?: string) => void;
  tabs_changed: (tabs: SessionInfo[]) => void;
  config: (config: Record<string, unknown>) => void;
  ready: () => void;
  error: (error: Error) => void;
  exit: (code: number | null) => void;
}

export class ITerm2Bridge extends EventEmitter {
  private process: ChildProcess | null = null;
  private socket: Socket | null = null;
  private buffer: string = "";
  private running = false;
  private restartTimer: ReturnType<typeof setTimeout> | null = null;

  /**
   * Start the Python bridge subprocess and connect via Unix socket.
   */
  async start(): Promise<void> {
    this.running = true;

    // Verify Python script exists
    console.log(`[iTerm2Bridge] Resolved script path: ${PYTHON_SCRIPT}`);
    if (!fs.existsSync(PYTHON_SCRIPT)) {
      throw new Error(
        `Python bridge script not found: ${PYTHON_SCRIPT}\n` +
          "Ensure iterm-bridge.py is in the mac-client directory.",
      );
    }

    // Clean up stale socket file
    try {
      fs.unlinkSync(SOCKET_PATH);
    } catch {
      /* ignore */
    }

    // Use venv Python if available, fall back to system python3
    const pythonBin = fs.existsSync(VENV_PYTHON) ? VENV_PYTHON : "python3";
    console.log(`[iTerm2Bridge] Using Python: ${pythonBin}`);

    // Launch Python subprocess
    this.process = spawn(pythonBin, [PYTHON_SCRIPT, SOCKET_PATH], {
      stdio: ["pipe", "pipe", "pipe"],
    });

    this.process.stdout?.on("data", (data: Buffer) => {
      console.log("[iTerm2Bridge] Python stdout:", data.toString().trim());
    });

    this.process.stderr?.on("data", (data: Buffer) => {
      const msg = data.toString().trim();
      // Check for missing iterm2 package
      if (msg.includes("ModuleNotFoundError") && msg.includes("iterm2")) {
        console.error(
          "[iTerm2Bridge] Python iterm2 package not installed.\n" +
            "Install it with: cd mac-client && python3 -m venv .venv && .venv/bin/pip install -r requirements.txt",
        );
      }
      console.error("[iTerm2Bridge] Python stderr:", msg);
    });

    this.process.on("exit", (code) => {
      console.log(`[iTerm2Bridge] Python process exited with code ${code}`);
      this.socket?.destroy();
      this.socket = null;
      this.process = null;
      this.emit("exit", code);

      // Auto-restart if still supposed to be running
      if (this.running) {
        console.log(`[iTerm2Bridge] Restarting in ${RESTART_DELAY_MS}ms...`);
        this.restartTimer = setTimeout(() => {
          this.restartTimer = null;
          if (this.running)
            this.start().catch((err) => {
              console.error("[iTerm2Bridge] Restart failed:", err);
              this.emit(
                "error",
                err instanceof Error ? err : new Error(String(err)),
              );
            });
        }, RESTART_DELAY_MS);
      }
    });

    // Wait for Python to set up the socket, then connect
    await this.connectToSocket();
  }

  /**
   * Connect to the Python bridge's Unix domain socket with retries.
   */
  private async connectToSocket(): Promise<void> {
    for (let i = 0; i < MAX_CONNECT_RETRIES; i++) {
      try {
        await new Promise<void>((resolve, reject) => {
          const socket = createConnection(SOCKET_PATH, () => {
            this.socket = socket;
            this.setupSocketHandlers();
            resolve();
          });
          socket.on("error", reject);
        });
        console.log("[iTerm2Bridge] Connected to Python bridge socket");
        return;
      } catch {
        await new Promise((r) => setTimeout(r, CONNECT_RETRY_DELAY_MS));
      }
    }
    throw new Error("Failed to connect to Python bridge socket after retries");
  }

  /**
   * Set up handlers for incoming data from Python bridge.
   */
  private setupSocketHandlers(): void {
    if (!this.socket) return;

    this.socket.on("data", (data: Buffer) => {
      this.buffer += data.toString("utf-8");
      // JSON lines protocol: split on newlines
      let newlineIndex: number;
      while ((newlineIndex = this.buffer.indexOf("\n")) !== -1) {
        const line = this.buffer.slice(0, newlineIndex);
        this.buffer = this.buffer.slice(newlineIndex + 1);
        if (line.trim()) {
          try {
            const msg = JSON.parse(line);
            this.handleMessage(msg);
          } catch {
            console.error("[iTerm2Bridge] Failed to parse message:", line);
          }
        }
      }
    });

    this.socket.on("close", () => {
      console.log("[iTerm2Bridge] Socket closed");
      this.socket = null;
    });

    this.socket.on("error", (err: Error) => {
      console.error("[iTerm2Bridge] Socket error:", err.message);
      this.emit("error", err);
    });
  }

  /**
   * Handle a parsed message from the Python bridge.
   */
  private handleMessage(msg: Record<string, unknown>): void {
    switch (msg.type) {
      case "terminal_data":
      case "initial_terminal_data": {
        // Decode base64 terminal data
        const termData = Buffer.from(msg.data as string, "base64");
        // Emit as the original message type for proper routing
        this.emit("terminal_data", msg.session_id as string, termData, msg.type);
        break;
      }
      case "sessions":
        this.emit("sessions", msg.sessions as SessionInfo[]);
        break;
      case "tab_switched":
        this.emit(
          "tab_switched",
          msg.tab_id as string,
          msg.session_id as string | undefined,
        );
        break;
      case "tabs_changed":
        this.emit("tabs_changed", msg.tabs as SessionInfo[]);
        break;
      case "config":
        this.emit("config", msg);
        break;
      case "ready":
        this.emit("ready");
        break;
      case "error":
        this.emit("error", new Error(msg.message as string));
        break;
      default:
        console.warn("[iTerm2Bridge] Unknown message type:", msg.type);
    }
  }

  /**
   * Send a command to the Python bridge.
   */
  send(msg: Record<string, unknown>): void {
    if (this.socket && !this.socket.destroyed) {
      const line = JSON.stringify(msg) + "\n";
      this.socket.write(line);
    }
  }

  /**
   * Send terminal input to a specific iTerm2 session.
   */
  sendInput(sessionId: string, data: string): void {
    this.send({
      type: "terminal_input",
      session_id: sessionId,
      data: Buffer.from(data).toString("base64"),
    });
  }

  /**
   * Send terminal resize to a specific iTerm2 session.
   */
  sendResize(sessionId: string, cols: number, rows: number): void {
    this.send({
      type: "terminal_resize",
      session_id: sessionId,
      cols,
      rows,
    });
  }

  /**
   * Request tab switch.
   */
  switchTab(tabId: string): void {
    this.send({ type: "tab_switch", tab_id: tabId });
  }

  /**
   * Request new tab creation.
   */
  createTab(): void {
    this.send({ type: "tab_create" });
  }

  /**
   * Request tab close.
   */
  closeTab(tabId: string): void {
    this.send({ type: "tab_close", tab_id: tabId });
  }

  /**
   * Request screen content refresh for a session.
   */
  requestScreenRefresh(sessionId?: string): void {
    this.send({ type: "request_screen_refresh", session_id: sessionId });
  }

  /**
   * Request full state resend (sessions, config, screen content).
   * Used when browser reconnects.
   */
  resendInitialState(): void {
    this.send({ type: "resend_initial_state" });
  }

  /**
   * Stop the bridge and clean up.
   */
  async stop(): Promise<void> {
    this.running = false;

    // Cancel any pending restart
    if (this.restartTimer) {
      clearTimeout(this.restartTimer);
      this.restartTimer = null;
    }

    this.socket?.destroy();
    this.socket = null;

    if (this.process) {
      this.process.kill("SIGTERM");
      this.process = null;
    }

    // Clean up socket file
    try {
      fs.unlinkSync(SOCKET_PATH);
    } catch {
      /* ignore */
    }
  }
}
