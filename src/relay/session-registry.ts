/**
 * Session Registry - Maps session codes to WebSocket connections.
 * Handles session creation, joining, and cleanup.
 */

import type { WebSocket } from 'ws';
import { generateSessionCode } from '../shared/session-codes.js';
import { SESSION_CODE_EXPIRY_MS } from '../shared/constants.js';

/**
 * Represents a paired session between Mac and Browser clients
 */
export interface Session {
  code: string;
  sessionId: string;
  mac: WebSocket | null;
  browser: WebSocket | null;
  createdAt: number;
  expiresAt: number;
}

/**
 * Result of attempting to join a session
 */
export type JoinResult =
  | { success: true; session: Session }
  | { success: false; error: 'INVALID_CODE' | 'EXPIRED_CODE' | 'ALREADY_JOINED' };

/**
 * Manages session lifecycles and connection pairing.
 * Uses Map for O(1) code lookups and WeakMap concepts for cleanup.
 */
export class SessionRegistry {
  private sessions = new Map<string, Session>();
  private macToSession = new Map<WebSocket, Session>();
  private browserToSession = new Map<WebSocket, Session>();
  private cleanupInterval: ReturnType<typeof setInterval> | null = null;

  constructor() {
    // Start periodic cleanup of expired sessions
    this.startCleanupInterval();
  }

  /**
   * Create a new session for a Mac client.
   * Generates a unique session code and stores the connection.
   */
  createSession(macSocket: WebSocket): Session {
    // Generate unique code (collision unlikely but check anyway)
    let code: string;
    do {
      code = generateSessionCode();
    } while (this.sessions.has(code));

    const now = Date.now();
    const session: Session = {
      code,
      sessionId: crypto.randomUUID(),
      mac: macSocket,
      browser: null,
      createdAt: now,
      expiresAt: now + SESSION_CODE_EXPIRY_MS,
    };

    this.sessions.set(code, session);
    this.macToSession.set(macSocket, session);

    return session;
  }

  /**
   * Look up a session by its code.
   * Returns null if not found (does not check expiry).
   */
  getSession(code: string): Session | null {
    return this.sessions.get(code) ?? null;
  }

  /**
   * Attempt to join a session with a browser client.
   * Validates code exists, hasn't expired, and isn't already joined.
   */
  joinSession(code: string, browserSocket: WebSocket): JoinResult {
    const session = this.sessions.get(code);

    if (!session) {
      return { success: false, error: 'INVALID_CODE' };
    }

    if (Date.now() > session.expiresAt) {
      // Clean up expired session
      this.removeSession(code);
      return { success: false, error: 'EXPIRED_CODE' };
    }

    if (session.browser !== null) {
      return { success: false, error: 'ALREADY_JOINED' };
    }

    // Join successful - link browser to session
    session.browser = browserSocket;
    this.browserToSession.set(browserSocket, session);

    // Extend expiry now that we're connected (or remove expiry entirely)
    // For now, we'll let connected sessions live until disconnect
    session.expiresAt = Infinity;

    return { success: true, session };
  }

  /**
   * Remove a session by code.
   * Called on Mac disconnect or explicit cleanup.
   */
  removeSession(code: string): void {
    const session = this.sessions.get(code);
    if (!session) return;

    // Clean up all references
    if (session.mac) {
      this.macToSession.delete(session.mac);
    }
    if (session.browser) {
      this.browserToSession.delete(session.browser);
    }
    this.sessions.delete(code);
  }

  /**
   * Find session associated with a Mac socket.
   * Used for message routing and cleanup.
   */
  findSessionByMac(socket: WebSocket): Session | null {
    return this.macToSession.get(socket) ?? null;
  }

  /**
   * Find session associated with a Browser socket.
   * Used for message routing and cleanup.
   */
  findSessionByBrowser(socket: WebSocket): Session | null {
    return this.browserToSession.get(socket) ?? null;
  }

  /**
   * Disconnect browser from session without removing session.
   * Mac can still send session code to new browser.
   */
  disconnectBrowser(socket: WebSocket): void {
    const session = this.browserToSession.get(socket);
    if (session) {
      session.browser = null;
      // Reset expiry since browser disconnected
      session.expiresAt = Date.now() + SESSION_CODE_EXPIRY_MS;
      this.browserToSession.delete(socket);
    }
  }

  /**
   * Clean up expired sessions.
   * Returns count of sessions removed.
   */
  cleanupExpired(): number {
    const now = Date.now();
    let count = 0;

    for (const [code, session] of this.sessions) {
      if (now > session.expiresAt) {
        this.removeSession(code);
        count++;
      }
    }

    return count;
  }

  /**
   * Get current session count (for monitoring).
   */
  get size(): number {
    return this.sessions.size;
  }

  /**
   * Start the cleanup interval.
   */
  private startCleanupInterval(): void {
    // Run cleanup every minute
    this.cleanupInterval = setInterval(() => {
      const cleaned = this.cleanupExpired();
      if (cleaned > 0) {
        console.log(`[Registry] Cleaned up ${cleaned} expired session(s)`);
      }
    }, 60 * 1000);

    // Don't keep Node.js running just for cleanup
    this.cleanupInterval.unref?.();
  }

  /**
   * Stop the cleanup interval (for graceful shutdown).
   */
  destroy(): void {
    if (this.cleanupInterval) {
      clearInterval(this.cleanupInterval);
      this.cleanupInterval = null;
    }
    this.sessions.clear();
    this.macToSession.clear();
    this.browserToSession.clear();
  }
}

/**
 * Singleton instance for use by the relay server.
 * Export class for testing/multiple instances if needed.
 */
export const sessionRegistry = new SessionRegistry();
