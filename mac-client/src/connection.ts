/**
 * WebSocket connection manager for Mac client.
 * Handles connection lifecycle, reconnection with exponential backoff,
 * and session code management.
 */

import WebSocket from 'ws';
import { randomUUID } from 'crypto';
import { ConnectionState, canTransition } from './state-machine.js';

// Reconnection configuration
const INITIAL_DELAY_MS = 1000;
const MAX_DELAY_MS = 30000;
const BACKOFF_MULTIPLIER = 2;
const JITTER_FACTOR = 0.1;

export interface ConnectionEvents {
  onCodeReceived: (code: string) => void;
  onStateChange?: (state: ConnectionState) => void;
  onError?: (error: Error) => void;
  onMessage?: (data: string) => void;
}

export class ConnectionManager {
  private ws: WebSocket | null = null;
  private state: ConnectionState = 'disconnected';
  private attempt: number = 0;
  private sessionCode: string | null = null;
  private readonly clientId: string;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(
    private readonly relayUrl: string,
    private readonly events: ConnectionEvents
  ) {
    this.clientId = randomUUID();
  }

  /**
   * Initiate connection to relay server.
   */
  connect(): void {
    if (this.state !== 'disconnected' && this.state !== 'reconnecting') {
      console.warn(`[Connection] Cannot connect from state: ${this.state}`);
      return;
    }

    if (this.state === 'reconnecting') {
      this.transition('connecting');
    } else {
      this.transition('connecting');
    }

    try {
      this.ws = new WebSocket(this.relayUrl);

      this.ws.on('open', () => this.handleOpen());
      this.ws.on('message', (data: Buffer) => this.handleMessage(data));
      this.ws.on('close', (code: number, reason: Buffer) => this.handleClose(code, reason.toString()));
      this.ws.on('error', (err: Error) => this.handleError(err));
    } catch (err) {
      console.error('[Connection] Failed to create WebSocket:', err);
      this.scheduleReconnect();
    }
  }

  /**
   * Handle WebSocket open event.
   * Sends registration message to relay.
   */
  private handleOpen(): void {
    this.transition('authenticating');
    this.attempt = 0; // Reset attempt counter on successful connection

    const registerMessage = JSON.stringify({
      type: 'register',
      clientId: this.clientId,
    });

    this.ws?.send(registerMessage);
    console.log('[Connection] Sent registration request');
  }

  /**
   * Handle incoming WebSocket messages.
   */
  private handleMessage(data: Buffer): void {
    try {
      const message = JSON.parse(data.toString());

      switch (message.type) {
        case 'registered':
          this.sessionCode = message.code;
          this.transition('connected');
          this.events.onCodeReceived(message.code);
          break;

        case 'error':
          console.error(`[Connection] Relay error: ${message.code} - ${message.message}`);
          this.events.onError?.(new Error(`${message.code}: ${message.message}`));
          break;

        case 'pong':
          // Heartbeat response - handled silently
          break;

        default:
          // Forward all other message types to the session manager
          this.events.onMessage?.(data.toString());
      }
    } catch (err) {
      console.error('[Connection] Failed to parse message:', err);
    }
  }

  /**
   * Handle WebSocket close event.
   * Initiates reconnection with backoff.
   */
  private handleClose(code: number, reason: string): void {
    console.log(`[Connection] Closed: ${code} ${reason || '(no reason)'}`);
    this.ws = null;

    // Don't reconnect if we're already disconnected (intentional shutdown)
    if (this.state === 'disconnected') {
      return;
    }

    this.scheduleReconnect();
  }

  /**
   * Handle WebSocket errors.
   */
  private handleError(err: Error): void {
    console.error('[Connection] WebSocket error:', err.message);
    this.events.onError?.(err);
    // Note: error events are typically followed by close events,
    // so we let handleClose trigger reconnection
  }

  /**
   * Schedule a reconnection attempt with exponential backoff.
   */
  private scheduleReconnect(): void {
    if (this.state === 'disconnected') {
      return; // Don't reconnect if intentionally disconnected
    }

    this.transition('reconnecting');
    const delay = this.calculateBackoff();
    this.attempt++;

    console.log(`[Connection] Reconnecting in ${delay}ms (attempt ${this.attempt})`);

    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connect();
    }, delay);
  }

  /**
   * Calculate backoff delay with jitter.
   * Formula: min(initialDelay * multiplier^attempt, maxDelay) + jitter
   */
  private calculateBackoff(): number {
    const exponentialDelay = INITIAL_DELAY_MS * Math.pow(BACKOFF_MULTIPLIER, this.attempt);
    const cappedDelay = Math.min(exponentialDelay, MAX_DELAY_MS);
    const jitter = cappedDelay * JITTER_FACTOR * (Math.random() * 2 - 1);
    return Math.floor(cappedDelay + jitter);
  }

  /**
   * Transition to a new state if valid.
   */
  private transition(to: ConnectionState): boolean {
    if (!canTransition(this.state, to)) {
      console.warn(`[Connection] Invalid transition: ${this.state} -> ${to}`);
      return false;
    }
    console.log(`[Connection] ${this.state} -> ${to}`);
    this.state = to;
    this.events.onStateChange?.(to);
    return true;
  }

  /**
   * Gracefully disconnect from relay.
   */
  disconnect(): void {
    console.log('[Connection] Disconnecting...');

    // Cancel any pending reconnect
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    // Close WebSocket if open
    if (this.ws) {
      this.ws.close(1000, 'Client shutdown');
      this.ws = null;
    }

    this.state = 'disconnected';
    this.sessionCode = null;
    console.log('[Connection] Disconnected');
  }

  /**
   * Get current connection state.
   */
  getState(): ConnectionState {
    return this.state;
  }

  /**
   * Get current session code (if connected).
   */
  getSessionCode(): string | null {
    return this.sessionCode;
  }

  /**
   * Get unique client identifier.
   */
  getClientId(): string {
    return this.clientId;
  }

  /**
   * Send a raw message to the relay server.
   * Used by SessionManager to forward terminal data and tab messages.
   */
  send(data: string): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(data);
    }
  }
}
