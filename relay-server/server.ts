/**
 * WebSocket Relay Server
 *
 * Handles connections from Mac clients (/mac) and browsers (/browser),
 * routes messages between paired connections via session codes.
 */

import { WebSocketServer, WebSocket, type RawData } from 'ws';
import type { IncomingMessage } from 'http';
import { sessionRegistry, type Session } from './session-registry.js';
import { parseMessage, type IncomingMessage as ProtocolMessage } from './shared/protocol.js';
import {
  DEFAULT_RELAY_PORT,
  HEARTBEAT_INTERVAL_MS,
} from './shared/constants.js';

// =============================================================================
// Types
// =============================================================================

interface ConnectionHealth {
  isAlive: boolean;
}

// Track pending join for browser connections
interface BrowserState {
  joinedSession: Session | null;
}

// =============================================================================
// Server Setup
// =============================================================================

const PORT = process.env.RELAY_PORT ? parseInt(process.env.RELAY_PORT, 10) : DEFAULT_RELAY_PORT;

const wss = new WebSocketServer({ port: PORT });

// Track connection health for heartbeat
const connectionHealth = new WeakMap<WebSocket, ConnectionHealth>();

// Track browser state (pending join)
const browserState = new WeakMap<WebSocket, BrowserState>();

console.log(`[Relay] Server listening on port ${PORT}`);

// =============================================================================
// Connection Handling
// =============================================================================

wss.on('connection', (ws: WebSocket, req: IncomingMessage) => {
  const path = req.url;

  if (path === '/mac') {
    handleMacConnection(ws);
  } else if (path === '/browser') {
    handleBrowserConnection(ws);
  } else {
    console.log(`[Relay] Invalid path: ${path}`);
    ws.close(4000, 'Invalid path');
  }
});

// =============================================================================
// Mac Client Handler
// =============================================================================

function handleMacConnection(ws: WebSocket): void {
  // Initialize health tracking
  connectionHealth.set(ws, { isAlive: true });

  // Create session immediately and send code
  const session = sessionRegistry.createSession(ws);

  console.log(`[Relay] Mac connected, session code: ${session.code}`);

  // Send registered message with code
  sendJson(ws, {
    type: 'registered',
    code: session.code,
    expiresAt: session.expiresAt,
  });

  // Handle messages
  ws.on('message', (data: RawData) => {
    const raw = data.toString();
    const result = parseMessage(raw);

    if (result.success === false) {
      console.warn(`[Relay] Invalid message from Mac: ${result.error}`);
      sendJson(ws, {
        type: 'error',
        code: 'INVALID_MESSAGE',
        message: result.error,
      });
      return;
    }

    const msg = result.data;
    const macSession = sessionRegistry.findSessionByMac(ws);
    const hasBrowser = macSession?.browser?.readyState === WebSocket.OPEN;
    // Only log non-terminal messages to reduce noise
    if (msg.type !== 'terminal_data' && msg.type !== 'initial_terminal_data') {
      console.log(`[Relay] Mac → ${msg.type} (browser=${hasBrowser ? 'connected' : 'none'})`);
    }

    switch (msg.type) {
      case 'data':
        // Forward data to browser if connected
        if (macSession?.browser?.readyState === WebSocket.OPEN) {
          macSession.browser.send(raw);
        }
        break;

      case 'ping':
        // Respond with pong
        sendJson(ws, { type: 'pong', ts: msg.ts });
        break;

      case 'register':
        // Mac is already registered - ignore duplicate register
        console.log(`[Relay] Mac re-registered (ignored), clientId: ${msg.clientId}`);
        break;

      default: {
        // Forward any valid parsed message to paired browser
        // (terminal_data, tab_list, tab_created, tab_closed, tab_switch, config, etc.)
        if (macSession?.browser?.readyState === WebSocket.OPEN) {
          macSession.browser.send(raw);
          // Only log non-terminal_data messages to reduce noise
          if (msg.type !== 'terminal_data' && msg.type !== 'initial_terminal_data') {
            console.log(`[Relay] Forwarded ${msg.type} to browser`);
          }
        } else if (msg.type !== 'terminal_data' && msg.type !== 'initial_terminal_data') {
          console.log(`[Relay] Dropped ${msg.type} (no browser)`);
        }
      }
    }
  });

  // Handle pong for heartbeat
  ws.on('pong', () => {
    const health = connectionHealth.get(ws);
    if (health) health.isAlive = true;
  });

  // Handle disconnect
  ws.on('close', () => {
    const session = sessionRegistry.findSessionByMac(ws);
    if (session) {
      console.log(`[Relay] Mac disconnected, session code: ${session.code}`);

      // Notify browser if connected
      if (session.browser?.readyState === WebSocket.OPEN) {
        sendJson(session.browser, {
          type: 'error',
          code: 'MAC_DISCONNECTED',
          message: 'Mac client disconnected',
        });
      }

      // Remove session
      sessionRegistry.removeSession(session.code);
    }
  });

  ws.on('error', (err) => {
    console.error(`[Relay] Mac WebSocket error:`, err.message);
  });
}

// =============================================================================
// Browser Client Handler
// =============================================================================

function handleBrowserConnection(ws: WebSocket): void {
  // Initialize health tracking
  connectionHealth.set(ws, { isAlive: true });
  browserState.set(ws, { joinedSession: null });

  console.log(`[Relay] Browser connected, waiting for join...`);

  // Handle messages
  ws.on('message', (data: RawData) => {
    const raw = data.toString();
    const result = parseMessage(raw);

    if (result.success === false) {
      console.warn(`[Relay] Invalid message from Browser: ${result.error}`);
      sendJson(ws, {
        type: 'error',
        code: 'INVALID_MESSAGE',
        message: result.error,
      });
      return;
    }

    const msg = result.data;
    const state = browserState.get(ws)!;
    const hasMac = state.joinedSession?.mac?.readyState === WebSocket.OPEN;
    if (msg.type !== 'join') {
      console.log(`[Relay] Browser → ${msg.type} (mac=${hasMac ? 'connected' : 'none'})`);
    }

    switch (msg.type) {
      case 'join':
        handleJoin(ws, msg.code, state);
        break;

      case 'rejoin':
        handleRejoin(ws, msg.sessionId, state);
        break;

      case 'data':
        // Forward data to Mac if joined
        if (state.joinedSession?.mac?.readyState === WebSocket.OPEN) {
          state.joinedSession.mac.send(raw);
        }
        break;

      case 'ping':
        // Respond with pong
        sendJson(ws, { type: 'pong', ts: msg.ts });
        break;

      case 'register':
        // Browser can't register - only Mac can
        sendJson(ws, {
          type: 'error',
          code: 'INVALID_MESSAGE',
          message: 'Browser cannot register, use join instead',
        });
        break;

      default: {
        // Forward any valid parsed message to paired Mac
        // (terminal_input, terminal_resize, tab_switch, tab_create, tab_close, etc.)
        if (state.joinedSession?.mac?.readyState === WebSocket.OPEN) {
          state.joinedSession.mac.send(raw);
          console.log(`[Relay] Forwarded ${msg.type} to Mac`);
        } else {
          console.log(`[Relay] Dropped ${msg.type} (no Mac)`);
        }
      }
    }
  });

  // Handle pong for heartbeat
  ws.on('pong', () => {
    const health = connectionHealth.get(ws);
    if (health) health.isAlive = true;
  });

  // Handle disconnect
  ws.on('close', () => {
    const state = browserState.get(ws);
    if (state?.joinedSession) {
      console.log(`[Relay] Browser disconnected from session: ${state.joinedSession.code}`);
      sessionRegistry.disconnectBrowser(ws);
    } else {
      console.log(`[Relay] Browser disconnected (never joined)`);
    }
  });

  ws.on('error', (err) => {
    console.error(`[Relay] Browser WebSocket error:`, err.message);
  });
}

/**
 * Handle browser join attempt
 */
function handleJoin(ws: WebSocket, code: string, state: BrowserState): void {
  if (state.joinedSession) {
    sendJson(ws, {
      type: 'error',
      code: 'ALREADY_JOINED',
      message: 'Already joined a session',
    });
    return;
  }

  const result = sessionRegistry.joinSession(code, ws);

  if (result.success === false) {
    console.log(`[Relay] Browser join failed: ${result.error}, code: ${code}`);
    sendJson(ws, {
      type: 'error',
      code: result.error,
      message: getErrorMessage(result.error),
    });
    return;
  }

  state.joinedSession = result.session;
  console.log(`[Relay] Browser joined session: ${code}`);

  sendJson(ws, {
    type: 'joined',
    sessionId: result.session.sessionId,
  });

  // Notify the Mac that a browser has connected so it can send initial state
  // (tab_list, config, terminal content redraw)
  if (result.session.mac?.readyState === WebSocket.OPEN) {
    console.log(`[Relay] Notifying Mac that browser connected`);
    sendJson(result.session.mac, {
      type: 'browser_connected',
      sessionId: result.session.sessionId,
    });
  }
}

function getErrorMessage(code: 'INVALID_CODE' | 'EXPIRED_CODE' | 'ALREADY_JOINED'): string {
  switch (code) {
    case 'INVALID_CODE':
      return 'Session code not found';
    case 'EXPIRED_CODE':
      return 'Session code has expired';
    case 'ALREADY_JOINED':
      return 'Session already has a browser connected';
  }
}

/**
 * Handle browser rejoin attempt (after page refresh)
 */
function handleRejoin(ws: WebSocket, sessionId: string, state: BrowserState): void {
  if (state.joinedSession) {
    sendJson(ws, {
      type: 'error',
      code: 'ALREADY_JOINED',
      message: 'Already joined a session',
    });
    return;
  }

  const result = sessionRegistry.rejoinSession(sessionId, ws);

  if (result.success === false) {
    console.log(`[Relay] Browser rejoin failed: ${result.error}, sessionId: ${sessionId.slice(0, 8)}...`);
    sendJson(ws, {
      type: 'error',
      code: result.error,
      message: result.error === 'SESSION_NOT_FOUND'
        ? 'Session not found'
        : 'Mac client disconnected',
    });
    return;
  }

  state.joinedSession = result.session;
  console.log(`[Relay] Browser rejoined session: ${result.session.code}`);

  sendJson(ws, {
    type: 'joined',
    sessionId: result.session.sessionId,
  });

  // Notify the Mac that a browser has reconnected so it can send current state
  if (result.session.mac?.readyState === WebSocket.OPEN) {
    console.log(`[Relay] Notifying Mac that browser reconnected`);
    sendJson(result.session.mac, {
      type: 'browser_connected',
      sessionId: result.session.sessionId,
    });
  }
}

// =============================================================================
// Heartbeat
// =============================================================================

const heartbeatInterval = setInterval(() => {
  wss.clients.forEach((ws) => {
    const health = connectionHealth.get(ws);
    if (!health || !health.isAlive) {
      console.log('[Relay] Terminating dead connection');
      return ws.terminate();
    }
    health.isAlive = false;
    ws.ping();
  });
}, HEARTBEAT_INTERVAL_MS);

// Don't keep Node.js running just for heartbeat if everything else stops
heartbeatInterval.unref?.();

// =============================================================================
// Graceful Shutdown
// =============================================================================

wss.on('close', () => {
  clearInterval(heartbeatInterval);
  sessionRegistry.destroy();
  console.log('[Relay] Server closed');
});

// Handle process termination
process.on('SIGTERM', () => {
  console.log('[Relay] SIGTERM received, closing...');
  wss.close();
});

process.on('SIGINT', () => {
  console.log('[Relay] SIGINT received, closing...');
  wss.close();
});

// =============================================================================
// Utilities
// =============================================================================

function sendJson(ws: WebSocket, data: object): void {
  if (ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(data));
  }
}

// =============================================================================
// Exports for testing/integration
// =============================================================================

export { wss, PORT };
