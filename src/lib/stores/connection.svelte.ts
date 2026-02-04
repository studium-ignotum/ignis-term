/**
 * WebSocket connection store for browser-to-relay communication.
 * Uses reconnecting-websocket for automatic reconnection.
 * Routes terminal_data and config messages to the terminal store.
 */

import ReconnectingWebSocket, { type ErrorEvent as RWSErrorEvent } from 'reconnecting-websocket';
import type {
  JoinMessage,
  JoinedMessage,
  ErrorMessage,
  DataMessage,
  TerminalDataMessage,
  TerminalInputMessage,
  TerminalResizeMessage,
  ConfigMessage,
  TabListMessage,
  TabSwitchMessage,
  TabCreatedMessage,
  TabClosedMessage,
} from '../../shared/protocol';
import { terminalStore } from './terminal.svelte';
import { tabsStore } from './tabs.svelte';

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
let sessionId = $state<string | null>(null);
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
  get sessionId() { return sessionId; },
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
        sessionId = msg.sessionId;
        error = null;
        // Set the active terminal session
        terminalStore.setActiveSession(msg.sessionId);
        break;
      }

      case 'error': {
        const msg = data as ErrorMessage;
        console.error('[Connection] Error:', msg.code, msg.message);
        error = msg.message;
        state = 'disconnected';
        sessionId = null;
        // Close connection on auth error
        if (ws) {
          ws.close();
          ws = null;
        }
        break;
      }

      case 'data': {
        const msg = data as DataMessage;
        // Legacy generic data message - log for debugging
        console.log('[Connection] Data received:', msg.payload.length, 'bytes');
        break;
      }

      // -- Terminal I/O messages (Phase 2) ----------------------------------

      case 'terminal_data': {
        const msg = data as TerminalDataMessage;
        terminalStore.writeData(msg.sessionId, msg.payload);
        break;
      }

      case 'config': {
        const msg = data as ConfigMessage;
        console.log('[Connection] Config received:', msg.font, msg.fontSize);
        terminalStore.applyConfig(msg);
        break;
      }

      // -- Tab management messages ----------------------------------------------

      case 'tab_list': {
        const msg = data as TabListMessage;
        console.log('[Connection] Tab list received:', msg.tabs.length, 'tabs');
        tabsStore.setTabs(msg.tabs);
        break;
      }

      case 'tab_switch': {
        const msg = data as TabSwitchMessage;
        console.log('[Connection] Tab switch:', msg.tabId);
        tabsStore.handleTabSwitch(msg.tabId);
        break;
      }

      case 'tab_created': {
        const msg = data as TabCreatedMessage;
        console.log('[Connection] Tab created:', msg.tab.tabId, msg.tab.title);
        tabsStore.handleTabCreated(msg.tab);
        break;
      }

      case 'tab_closed': {
        const msg = data as TabClosedMessage;
        console.log('[Connection] Tab closed:', msg.tabId);
        tabsStore.handleTabClosed(msg.tabId);
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
  sessionId = null;
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
  sessionId = null;
  currentCode = null;
  terminalStore.setActiveSession(null);
  tabsStore.reset();
}

/**
 * Send a typed message to the relay server.
 * Used for terminal_input, terminal_resize, tab operations, etc.
 */
export function sendMessage(message: object): void {
  if (state === 'connected' && ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(message));
  }
}

/**
 * Send terminal input for a session.
 */
export function sendTerminalInput(termSessionId: string, payload: string): void {
  const msg: TerminalInputMessage = {
    type: 'terminal_input',
    sessionId: termSessionId,
    payload,
  };
  sendMessage(msg);
}

/**
 * Send terminal resize for a session.
 */
export function sendTerminalResize(termSessionId: string, cols: number, rows: number): void {
  const msg: TerminalResizeMessage = {
    type: 'terminal_resize',
    sessionId: termSessionId,
    cols,
    rows,
  };
  sendMessage(msg);
}

/**
 * Send data to the terminal (Mac client) - legacy generic data message
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
