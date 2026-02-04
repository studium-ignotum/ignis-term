/**
 * WebSocket connection store for browser-to-relay communication.
 * Uses reconnecting-websocket for automatic reconnection.
 */

import ReconnectingWebSocket, { type ErrorEvent as RWSErrorEvent } from 'reconnecting-websocket';
import type { JoinMessage, JoinedMessage, ErrorMessage, DataMessage } from '../../shared/protocol';

// =============================================================================
// Connection State Types
// =============================================================================

export type ConnectionState =
  | 'disconnected'
  | 'connecting'
  | 'authenticating'
  | 'connected'
  | 'reconnecting';

// =============================================================================
// Store State (Svelte 5 Runes)
// =============================================================================

let state = $state<ConnectionState>('disconnected');
let error = $state<string | null>(null);
let ws: ReconnectingWebSocket | null = null;
let currentCode: string | null = null;

// =============================================================================
// Exported Reactive Store
// =============================================================================

/**
 * Reactive connection store - use $connectionStore.state etc in components
 */
export const connectionStore = {
  get state() { return state; },
  get error() { return error; },
  get isConnected() { return state === 'connected'; }
};

// =============================================================================
// Event Handlers
// =============================================================================

function handleOpen(): void {
  console.log('[Connection] WebSocket opened, authenticating...');
  state = 'authenticating';

  // Send join message with session code
  if (ws && currentCode) {
    const joinMessage: JoinMessage = {
      type: 'join',
      code: currentCode
    };
    ws.send(JSON.stringify(joinMessage));
  }
}

function handleMessage(event: MessageEvent): void {
  try {
    const data = JSON.parse(event.data);

    switch (data.type) {
      case 'joined': {
        const msg = data as JoinedMessage;
        console.log('[Connection] Joined session:', msg.sessionId);
        state = 'connected';
        error = null;
        break;
      }

      case 'error': {
        const msg = data as ErrorMessage;
        console.error('[Connection] Error:', msg.code, msg.message);
        error = msg.message;
        state = 'disconnected';
        // Close connection on auth error
        if (ws) {
          ws.close();
          ws = null;
        }
        break;
      }

      case 'data': {
        const msg = data as DataMessage;
        // Emit to terminal - placeholder for Phase 2
        // Will dispatch a custom event or call a callback
        console.log('[Connection] Data received:', msg.payload.length, 'bytes');
        break;
      }

      default:
        console.log('[Connection] Unknown message type:', data.type);
    }
  } catch (e) {
    console.error('[Connection] Failed to parse message:', e);
  }
}

function handleClose(): void {
  console.log('[Connection] WebSocket closed');

  // If we were connected, transition to reconnecting
  // (reconnecting-websocket will handle the actual reconnection)
  if (state === 'connected') {
    state = 'reconnecting';
  } else if (state !== 'disconnected') {
    // Don't override explicit disconnection
    if (ws) {
      state = 'reconnecting';
    }
  }
}

function handleError(event: RWSErrorEvent): void {
  // Don't set error state here - reconnecting-websocket handles retries
  // Only log for debugging
  console.error('[Connection] WebSocket error:', event);
}

// =============================================================================
// Public API
// =============================================================================

/**
 * Connect to the relay server with a session code
 */
export function connect(sessionCode: string): void {
  // Close existing connection if any
  if (ws) {
    ws.close();
    ws = null;
  }

  state = 'connecting';
  error = null;
  currentCode = sessionCode;

  // Get relay URL from environment or use default
  const relayUrl = import.meta.env.VITE_RELAY_URL || 'ws://localhost:8080/browser';

  console.log('[Connection] Connecting to:', relayUrl);

  ws = new ReconnectingWebSocket(relayUrl, [], {
    maxReconnectionDelay: 30000,     // Max 30 seconds between retries
    minReconnectionDelay: 1000,      // Start with 1 second
    reconnectionDelayGrowFactor: 2,  // Double delay each retry
    maxRetries: 10,                  // Give up after 10 retries
  });

  ws.addEventListener('open', handleOpen);
  ws.addEventListener('message', handleMessage);
  ws.addEventListener('close', handleClose);
  ws.addEventListener('error', handleError);
}

/**
 * Disconnect from the relay server
 */
export function disconnect(): void {
  if (ws) {
    ws.close();
    ws = null;
  }
  state = 'disconnected';
  error = null;
  currentCode = null;
}

/**
 * Send data to the terminal (Mac client)
 * Used for keyboard input in Phase 2
 */
export function send(payload: string): void {
  if (state === 'connected' && ws && ws.readyState === WebSocket.OPEN) {
    const dataMessage: DataMessage = {
      type: 'data',
      payload
    };
    ws.send(JSON.stringify(dataMessage));
  }
}
