/**
 * WebSocket connection context for browser-to-relay communication.
 * Uses reconnecting-websocket for automatic reconnection.
 *
 * Protocol: v2 Rust relay
 * - Endpoint: /ws
 * - Auth: auth/auth_success/auth_failed
 * - Terminal I/O: Binary frames with session ID prefix
 */

import { createContext, useContext, useState, useRef, useCallback, useEffect, type ReactNode } from 'react';
import ReconnectingWebSocket from 'reconnecting-websocket';
import type {
  AuthMessage,
  AuthSuccessMessage,
  AuthFailedMessage,
  ErrorMessage,
  SessionConnectedMessage,
  SessionDisconnectedMessage,
  ConfigMessage,
} from '../../shared/protocol';
import { decodeBinaryFrame, encodeInputMessage, encodeResizeMessage } from '../protocol/binary';

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
// Session Storage Helpers
// =============================================================================

const SESSION_CODE_STORAGE_KEY = 'terminal-session-code';

function getStoredSessionCode(): string | null {
  try {
    return sessionStorage.getItem(SESSION_CODE_STORAGE_KEY);
  } catch {
    return null;
  }
}

function storeSessionCode(code: string): void {
  try {
    sessionStorage.setItem(SESSION_CODE_STORAGE_KEY, code);
  } catch {
    // Ignore storage errors
  }
}

function clearStoredSessionCode(): void {
  try {
    sessionStorage.removeItem(SESSION_CODE_STORAGE_KEY);
  } catch {
    // Ignore storage errors
  }
}

// =============================================================================
// Handler Types
// =============================================================================

/** Handler for JSON control messages (auth, config, session events) */
export type MessageHandler = (data: Record<string, unknown>) => void;

/** Handler for binary terminal data (session ID already decoded) */
export type BinaryHandler = (sessionId: string, data: Uint8Array) => void;

// =============================================================================
// Context Interface
// =============================================================================

interface ConnectionContextValue {
  state: ConnectionState;
  error: string | null;
  sessionCode: string | null;
  isConnected: boolean;
  connect: (sessionCode: string, onConnected?: () => void) => void;
  disconnect: () => void;
  /** Send a JSON control message */
  sendMessage: (message: object) => void;
  /** Send binary terminal input for a session */
  sendTerminalInput: (sessionId: string, payload: string) => void;
  /** Send terminal resize for a session */
  sendTerminalResize: (sessionId: string, cols: number, rows: number) => void;
  /** Send raw binary frame */
  sendBinary: (frame: Uint8Array) => void;
  /** Register handler for JSON control messages */
  registerMessageHandler: (handler: MessageHandler) => () => void;
  /** Register handler for binary terminal data */
  registerBinaryHandler: (handler: BinaryHandler) => () => void;
}

const ConnectionContext = createContext<ConnectionContextValue | null>(null);

export function useConnection(): ConnectionContextValue {
  const ctx = useContext(ConnectionContext);
  if (!ctx) throw new Error('useConnection must be used within ConnectionProvider');
  return ctx;
}

// =============================================================================
// Provider
// =============================================================================

export function ConnectionProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<ConnectionState>('disconnected');
  const [error, setError] = useState<string | null>(null);
  const [sessionCode, setSessionCode] = useState<string | null>(null);

  const wsRef = useRef<ReconnectingWebSocket | null>(null);
  const currentCodeRef = useRef<string | null>(null);
  const onConnectedCallbackRef = useRef<(() => void) | null>(null);
  const messageHandlersRef = useRef<Set<MessageHandler>>(new Set());
  const binaryHandlersRef = useRef<Set<BinaryHandler>>(new Set());

  // Refs for state values that event handlers need to read (avoids stale closures)
  const stateRef = useRef<ConnectionState>('disconnected');

  // ---------------------------------------------------------------------------
  // Handler Registration
  // ---------------------------------------------------------------------------

  const registerMessageHandler = useCallback((handler: MessageHandler) => {
    messageHandlersRef.current.add(handler);
    return () => { messageHandlersRef.current.delete(handler); };
  }, []);

  const registerBinaryHandler = useCallback((handler: BinaryHandler) => {
    binaryHandlersRef.current.add(handler);
    return () => { binaryHandlersRef.current.delete(handler); };
  }, []);

  // ---------------------------------------------------------------------------
  // Send Functions
  // ---------------------------------------------------------------------------

  const sendMessageFn = useCallback((message: object) => {
    const ws = wsRef.current;
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(message));
    }
  }, []);

  const sendBinary = useCallback((frame: Uint8Array) => {
    const ws = wsRef.current;
    if (stateRef.current === 'connected' && ws && ws.readyState === WebSocket.OPEN) {
      ws.send(frame);
    }
  }, []);

  const sendTerminalInput = useCallback((termSessionId: string, payload: string) => {
    const frame = encodeInputMessage(termSessionId, payload);
    sendBinary(frame);
  }, [sendBinary]);

  const sendTerminalResize = useCallback((termSessionId: string, cols: number, rows: number) => {
    const frame = encodeResizeMessage(termSessionId, cols, rows);
    sendBinary(frame);
  }, [sendBinary]);

  // ---------------------------------------------------------------------------
  // Disconnect
  // ---------------------------------------------------------------------------

  const disconnect = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
    setState('disconnected');
    stateRef.current = 'disconnected';
    setError(null);
    setSessionCode(null);
    currentCodeRef.current = null;
    clearStoredSessionCode();
    // Notify handlers of disconnect
    for (const handler of messageHandlersRef.current) {
      handler({ type: '__disconnect' });
    }
  }, []);

  // ---------------------------------------------------------------------------
  // Connect
  // ---------------------------------------------------------------------------

  const connect = useCallback((code: string, onConnected?: () => void) => {
    // Close existing connection if any
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    setState('connecting');
    stateRef.current = 'connecting';
    setError(null);
    setSessionCode(null);
    currentCodeRef.current = code;
    onConnectedCallbackRef.current = onConnected ?? null;

    // Derive relay URL: use env var in dev, or derive from current location in production
    const relayUrl = import.meta.env.VITE_RELAY_URL
      || `${location.protocol === 'https:' ? 'wss:' : 'ws:'}//${location.host}/ws`;

    const ws = new ReconnectingWebSocket(relayUrl, [], {
      maxReconnectionDelay: 30000,
      minReconnectionDelay: 1000,
      reconnectionDelayGrowFactor: 2,
      maxRetries: 10,
    });

    // Enable binary message handling
    ws.binaryType = 'arraybuffer';

    ws.addEventListener('open', () => {
      setState('authenticating');
      stateRef.current = 'authenticating';

      if (currentCodeRef.current) {
        // Send auth message with session code
        const authMessage: AuthMessage = {
          type: 'auth',
          session_code: currentCodeRef.current,
        };
        ws.send(JSON.stringify(authMessage));
      }
    });

    ws.addEventListener('message', (event: MessageEvent) => {
      // Binary frame: decode and dispatch to binary handlers
      if (event.data instanceof ArrayBuffer) {
        try {
          const frame = new Uint8Array(event.data);
          const { sessionId, payload } = decodeBinaryFrame(frame);
          for (const handler of binaryHandlersRef.current) {
            handler(sessionId, payload);
          }
        } catch (e) {
          console.error('[Connection] Failed to decode binary frame:', e);
        }
        return;
      }

      // Text frame: JSON control message
      try {
        const data = JSON.parse(event.data);

        switch (data.type) {
          case 'auth_success': {
            setState('connected');
            stateRef.current = 'connected';
            setSessionCode(currentCodeRef.current);
            setError(null);
            if (currentCodeRef.current) {
              storeSessionCode(currentCodeRef.current);
            }
            // Fire one-time connected callback
            if (onConnectedCallbackRef.current) {
              const cb = onConnectedCallbackRef.current;
              onConnectedCallbackRef.current = null;
              cb();
            }
            // Notify handlers
            for (const handler of messageHandlersRef.current) {
              handler(data);
            }
            break;
          }

          case 'auth_failed': {
            const msg = data as AuthFailedMessage;
            console.error('[Connection] Auth failed:', msg.reason);
            setError(msg.reason);
            setState('disconnected');
            stateRef.current = 'disconnected';
            setSessionCode(null);
            clearStoredSessionCode();
            if (wsRef.current) {
              wsRef.current.close();
              wsRef.current = null;
            }
            break;
          }

          case 'error': {
            const msg = data as ErrorMessage;
            console.error('[Connection] Error:', msg.code, msg.message);
            setError(msg.message);
            setState('disconnected');
            stateRef.current = 'disconnected';
            setSessionCode(null);
            clearStoredSessionCode();
            if (wsRef.current) {
              wsRef.current.close();
              wsRef.current = null;
            }
            break;
          }

          // Session events forwarded from mac-client
          case 'session_connected':
          case 'session_disconnected':
          // Config message
          case 'config':
          // Legacy tab messages (if any)
          case 'tab_list':
          case 'tab_switch':
          case 'tab_created':
          case 'tab_closed': {
            for (const handler of messageHandlersRef.current) {
              handler(data);
            }
            break;
          }

          default:
            console.log('[Connection] Unhandled message type:', data.type);
        }
      } catch (e) {
        console.error('[Connection] Failed to parse message:', e);
      }
    });

    ws.addEventListener('close', () => {
      if (stateRef.current === 'connected') {
        setState('reconnecting');
        stateRef.current = 'reconnecting';
      } else if (stateRef.current !== 'disconnected') {
        if (wsRef.current) {
          setState('reconnecting');
          stateRef.current = 'reconnecting';
        }
      }
    });

    ws.addEventListener('error', () => {
      // Error handling is done via close event
    });

    wsRef.current = ws;
  }, []);

  // Auto-reconnect on mount if we have a stored session code
  useEffect(() => {
    const stored = getStoredSessionCode();
    if (stored && stateRef.current === 'disconnected') {
      connect(stored);
    }
  }, [connect]);

  const value: ConnectionContextValue = {
    state,
    error,
    sessionCode,
    isConnected: state === 'connected',
    connect,
    disconnect,
    sendMessage: sendMessageFn,
    sendTerminalInput,
    sendTerminalResize,
    sendBinary,
    registerMessageHandler,
    registerBinaryHandler,
  };

  return (
    <ConnectionContext.Provider value={value}>
      {children}
    </ConnectionContext.Provider>
  );
}
