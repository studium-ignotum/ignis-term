/**
 * WebSocket connection context for browser-to-relay communication.
 * Uses reconnecting-websocket for automatic reconnection.
 * Routes terminal_data, config, and tab messages to registered handlers.
 */

import { createContext, useContext, useState, useRef, useCallback, useEffect, type ReactNode } from 'react';
import ReconnectingWebSocket, { type ErrorEvent as RWSErrorEvent } from 'reconnecting-websocket';
import type {
  JoinMessage,
  RejoinMessage,
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

// =============================================================================
// Connection State Types
// =============================================================================

export type ConnectionState =
  | 'disconnected'
  | 'connecting'
  | 'authenticating'
  | 'connected'
  | 'reconnecting'
  | 'rejoining';

// =============================================================================
// Session Storage Helpers
// =============================================================================

const SESSION_STORAGE_KEY = 'terminal-session-id';

function getStoredSessionId(): string | null {
  try {
    return sessionStorage.getItem(SESSION_STORAGE_KEY);
  } catch {
    return null;
  }
}

function storeSessionId(sessionId: string): void {
  try {
    sessionStorage.setItem(SESSION_STORAGE_KEY, sessionId);
  } catch {
    // Ignore storage errors
  }
}

function clearStoredSessionId(): void {
  try {
    sessionStorage.removeItem(SESSION_STORAGE_KEY);
  } catch {
    // Ignore storage errors
  }
}

export type MessageHandler = (data: Record<string, unknown>) => void;

interface ConnectionContextValue {
  state: ConnectionState;
  error: string | null;
  sessionId: string | null;
  isConnected: boolean;
  connect: (sessionCode: string, onConnected?: () => void) => void;
  disconnect: () => void;
  sendMessage: (message: object) => void;
  sendTerminalInput: (sessionId: string, payload: string) => void;
  sendTerminalResize: (sessionId: string, cols: number, rows: number) => void;
  sendScreenRefresh: (sessionId: string) => void;
  registerMessageHandler: (handler: MessageHandler) => () => void;
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
  const [sessionId, setSessionId] = useState<string | null>(null);

  const wsRef = useRef<ReconnectingWebSocket | null>(null);
  const currentCodeRef = useRef<string | null>(null);
  const onConnectedCallbackRef = useRef<(() => void) | null>(null);
  const messageHandlersRef = useRef<Set<MessageHandler>>(new Set());

  // We use refs for state values that event handlers need to read,
  // to avoid stale closure issues with WebSocket callbacks.
  const stateRef = useRef<ConnectionState>('disconnected');
  const rejoinAttemptedRef = useRef(false);
  const storedSessionIdRef = useRef<string | null>(null);

  const registerMessageHandler = useCallback((handler: MessageHandler) => {
    messageHandlersRef.current.add(handler);
    return () => { messageHandlersRef.current.delete(handler); };
  }, []);

  const sendMessageFn = useCallback((message: object) => {
    const ws = wsRef.current;
    if (stateRef.current === 'connected' && ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(message));
    }
  }, []);

  const sendTerminalInput = useCallback((termSessionId: string, payload: string) => {
    const msg: TerminalInputMessage = {
      type: 'terminal_input',
      sessionId: termSessionId,
      payload,
    };
    sendMessageFn(msg);
  }, [sendMessageFn]);

  const sendTerminalResize = useCallback((termSessionId: string, cols: number, rows: number) => {
    const msg: TerminalResizeMessage = {
      type: 'terminal_resize',
      sessionId: termSessionId,
      cols,
      rows,
    };
    sendMessageFn(msg);
  }, [sendMessageFn]);

  const sendScreenRefresh = useCallback((termSessionId: string) => {
    sendMessageFn({
      type: 'request_screen_refresh',
      sessionId: termSessionId,
    });
  }, [sendMessageFn]);

  const disconnect = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
    setState('disconnected');
    stateRef.current = 'disconnected';
    setError(null);
    setSessionId(null);
    currentCodeRef.current = null;
    clearStoredSessionId();
    // Notify handlers of disconnect (tabs/terminal will reset via their own effect)
    for (const handler of messageHandlersRef.current) {
      handler({ type: '__disconnect' });
    }
  }, []);

  const connect = useCallback((sessionCode: string, onConnected?: () => void) => {
    // Close existing connection if any
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    setState('connecting');
    stateRef.current = 'connecting';
    setError(null);
    setSessionId(null);
    currentCodeRef.current = sessionCode;
    onConnectedCallbackRef.current = onConnected ?? null;

    const relayUrl = import.meta.env.VITE_RELAY_URL || 'ws://localhost:8080/browser';

    const ws = new ReconnectingWebSocket(relayUrl, [], {
      maxReconnectionDelay: 30000,
      minReconnectionDelay: 1000,
      reconnectionDelayGrowFactor: 2,
      maxRetries: 10,
    });

    ws.addEventListener('open', () => {
      setState('authenticating');
      stateRef.current = 'authenticating';

      if (currentCodeRef.current) {
        const joinMessage: JoinMessage = {
          type: 'join',
          code: currentCodeRef.current,
        };
        ws.send(JSON.stringify(joinMessage));
      }
    });

    ws.addEventListener('message', (event: MessageEvent) => {
      try {
        const data = JSON.parse(event.data);

        switch (data.type) {
          case 'joined': {
            const msg = data as JoinedMessage;
            setState('connected');
            stateRef.current = 'connected';
            setSessionId(msg.sessionId);
            setError(null);
            storeSessionId(msg.sessionId);
            // Notify handlers (terminal context will set active session)
            for (const handler of messageHandlersRef.current) {
              handler(data);
            }
            // Fire one-time connected callback (used by login page for navigation)
            if (onConnectedCallbackRef.current) {
              const cb = onConnectedCallbackRef.current;
              onConnectedCallbackRef.current = null;
              cb();
            }
            break;
          }

          case 'error': {
            const msg = data as ErrorMessage;
            console.error('[Connection] Error:', msg.code, msg.message);
            setError(msg.message);
            setState('disconnected');
            stateRef.current = 'disconnected';
            setSessionId(null);
            // Clear stored session on certain errors
            if (msg.code === 'MAC_DISCONNECTED' || msg.code === 'SESSION_NOT_FOUND') {
              clearStoredSessionId();
            }
            if (wsRef.current) {
              wsRef.current.close();
              wsRef.current = null;
            }
            break;
          }

          case 'data':
            // Generic data message, currently unused
            break;

          // Terminal and tab messages are forwarded to registered handlers
          case 'terminal_data':
          case 'initial_terminal_data':
          case 'config':
          case 'tab_list':
          case 'tab_switch':
          case 'tab_created':
          case 'tab_closed': {
            for (const handler of messageHandlersRef.current) {
              handler(data);
            }
            break;
          }
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

  /**
   * Attempt to rejoin using a stored sessionId (after page refresh).
   * Uses ReconnectingWebSocket with limited retries for the initial attempt.
   */
  const attemptRejoin = useCallback((storedSessionId: string, onSuccess?: () => void) => {
    // Close existing connection if any
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    setState('rejoining');
    stateRef.current = 'rejoining';
    setError(null);
    storedSessionIdRef.current = storedSessionId;
    onConnectedCallbackRef.current = onSuccess ?? null;

    const relayUrl = import.meta.env.VITE_RELAY_URL || 'ws://localhost:8080/browser';

    const ws = new ReconnectingWebSocket(relayUrl, [], {
      maxReconnectionDelay: 5000,
      minReconnectionDelay: 1000,
      reconnectionDelayGrowFactor: 1.5,
      maxRetries: 3, // Limited retries for rejoin attempt
    });

    ws.addEventListener('open', () => {
      const rejoinMessage: RejoinMessage = {
        type: 'rejoin',
        sessionId: storedSessionId,
      };
      ws.send(JSON.stringify(rejoinMessage));
    });

    ws.addEventListener('message', (event: MessageEvent) => {
      try {
        const data = JSON.parse(event.data);

        switch (data.type) {
          case 'joined': {
            const msg = data as JoinedMessage;
            setState('connected');
            stateRef.current = 'connected';
            setSessionId(msg.sessionId);
            setError(null);
            storeSessionId(msg.sessionId);
            // Notify handlers (terminal context will set active session)
            for (const handler of messageHandlersRef.current) {
              handler(data);
            }
            // Fire one-time connected callback
            if (onConnectedCallbackRef.current) {
              const cb = onConnectedCallbackRef.current;
              onConnectedCallbackRef.current = null;
              cb();
            }
            break;
          }

          case 'error': {
            const msg = data as ErrorMessage;
            console.log('[Connection] Rejoin failed:', msg.code, msg.message);
            setState('disconnected');
            stateRef.current = 'disconnected';
            setSessionId(null);
            clearStoredSessionId();
            if (wsRef.current) {
              wsRef.current.close();
              wsRef.current = null;
            }
            break;
          }

          case 'data':
            // Generic data message, currently unused
            break;

          // Terminal and tab messages are forwarded to registered handlers
          case 'terminal_data':
          case 'initial_terminal_data':
          case 'config':
          case 'tab_list':
          case 'tab_switch':
          case 'tab_created':
          case 'tab_closed': {
            for (const handler of messageHandlersRef.current) {
              handler(data);
            }
            break;
          }
        }
      } catch (e) {
        console.error('[Connection] Failed to parse rejoin message:', e);
      }
    });

    ws.addEventListener('error', () => {
      // Error handling - if we can't connect at all, clear storage
      if (stateRef.current === 'rejoining') {
        setState('disconnected');
        stateRef.current = 'disconnected';
        clearStoredSessionId();
      }
    });

    ws.addEventListener('close', () => {
      if (stateRef.current === 'connected') {
        setState('reconnecting');
        stateRef.current = 'reconnecting';
      } else if (stateRef.current === 'rejoining') {
        // Rejoin failed, stay disconnected
        setState('disconnected');
        stateRef.current = 'disconnected';
        clearStoredSessionId();
      }
    });

    wsRef.current = ws;
  }, []);

  // Auto-rejoin on mount if we have a stored sessionId
  useEffect(() => {
    const stored = getStoredSessionId();
    if (stored && !rejoinAttemptedRef.current && stateRef.current === 'disconnected') {
      rejoinAttemptedRef.current = true;
      attemptRejoin(stored);
    }
  }, [attemptRejoin]);

  const value: ConnectionContextValue = {
    state,
    error,
    sessionId,
    isConnected: state === 'connected',
    connect,
    disconnect,
    sendMessage: sendMessageFn,
    sendTerminalInput,
    sendTerminalResize,
    sendScreenRefresh,
    registerMessageHandler,
  };

  return (
    <ConnectionContext.Provider value={value}>
      {children}
    </ConnectionContext.Provider>
  );
}
