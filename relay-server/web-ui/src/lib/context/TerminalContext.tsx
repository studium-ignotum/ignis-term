/**
 * Terminal state management context.
 *
 * Manages the active terminal session, xterm.js options, and a registry of
 * Terminal instances for routing incoming binary terminal data to the correct
 * terminal.
 *
 * Key features:
 * - Binary data routing via writeUtf8 for efficiency
 * - Data buffering before terminal mounts
 * - Per-session terminal instances (one xterm per session)
 * - Session resize forwarding (mac -> UI, one-way)
 */

import {
  createContext,
  useContext,
  useState,
  useRef,
  useCallback,
  useEffect,
  type ReactNode,
} from 'react';
import type { Terminal, ITerminalOptions } from '@xterm/xterm';
import type { ConfigMessage } from '../../shared/protocol';
import { defaultTerminalOptions, configToXtermOptions } from '../iterm-theme';
import { useConnection } from './ConnectionContext';

// Debug logging
const DEBUG = false;
const log = (...args: unknown[]) => DEBUG && console.log('[TerminalContext]', ...args);

// =============================================================================
// Context Types
// =============================================================================

/** Callback for session resize events (mac -> browser) */
type SessionResizeCallback = (cols: number, rows: number) => void;

interface TerminalContextValue {
  activeSessionId: string | null;
  options: ITerminalOptions;
  setActiveSession: (sessionId: string | null) => void;
  applyConfig: (config: ConfigMessage) => void;
  registerTerminal: (sessionId: string, terminal: Terminal) => void;
  unregisterTerminal: (sessionId: string) => void;
  /** Flush pending data and mark terminal as ready for direct writes (call after first fit) */
  markTerminalReady: (sessionId: string) => void;
  /** Write binary data to terminal (used internally by binary handler) */
  writeBinaryData: (sessionId: string, data: Uint8Array) => void;
  getTerminal: (sessionId: string) => Terminal | undefined;
  /** Subscribe to resize events for a specific session (mac -> browser). Returns unsubscribe fn. */
  onSessionResize: (sessionId: string, callback: SessionResizeCallback) => () => void;
}

const TerminalContext = createContext<TerminalContextValue | null>(null);

export function useTerminal(): TerminalContextValue {
  const ctx = useContext(TerminalContext);
  if (!ctx) throw new Error('useTerminal must be used within TerminalProvider');
  return ctx;
}

// =============================================================================
// Provider
// =============================================================================

export function TerminalProvider({ children }: { children: ReactNode }) {
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [options, setOptions] = useState<ITerminalOptions>({ ...defaultTerminalOptions });
  const terminalsRef = useRef<Map<string, Terminal>>(new Map());
  // Buffer binary data that arrives before terminal is fit (keyed by sessionId)
  const pendingDataRef = useRef<Map<string, Uint8Array[]>>(new Map());
  // Terminals that have been fit and are ready for direct writes
  const readyRef = useRef<Set<string>>(new Set());
  // Session resize listeners (mac -> browser)
  const resizeListenersRef = useRef<Map<string, Set<SessionResizeCallback>>>(new Map());

  const { registerMessageHandler, registerBinaryHandler } = useConnection();

  const setActiveSession = useCallback((sessionId: string | null) => {
    setActiveSessionId(sessionId);
  }, []);

  const applyConfig = useCallback((config: ConfigMessage) => {
    log('applyConfig:', config);
    setOptions(configToXtermOptions(config));
  }, []);

  /** Helper: write a chunk to a terminal using writeUtf8 when available. */
  const writeChunk = (terminal: Terminal, chunk: Uint8Array) => {
    const t = terminal as unknown as { writeUtf8?: (data: Uint8Array) => void };
    if (t.writeUtf8) {
      t.writeUtf8(chunk);
    } else {
      terminal.write(new TextDecoder().decode(chunk));
    }
  };

  /**
   * Register a terminal instance. Data is buffered until markTerminalReady()
   * is called (after the first FitAddon fit) so scrollback replays at the
   * correct terminal dimensions.
   */
  const registerTerminal = useCallback((sessionId: string, terminal: Terminal) => {
    log('registerTerminal:', sessionId);
    terminalsRef.current.set(sessionId, terminal);
  }, []);

  /**
   * Mark a terminal as ready (call after first successful FitAddon.fit()).
   * Flushes any pending data at the now-correct terminal size.
   */
  const markTerminalReady = useCallback((sessionId: string) => {
    log('markTerminalReady:', sessionId);
    readyRef.current.add(sessionId);

    const terminal = terminalsRef.current.get(sessionId);
    const pending = pendingDataRef.current.get(sessionId);
    if (terminal && pending && pending.length > 0) {
      log('markTerminalReady: flushing', pending.length, 'buffered chunks');
      for (const chunk of pending) {
        writeChunk(terminal, chunk);
      }
    }
    pendingDataRef.current.delete(sessionId);
  }, []);

  const unregisterTerminal = useCallback((sessionId: string) => {
    terminalsRef.current.delete(sessionId);
    readyRef.current.delete(sessionId);
    resizeListenersRef.current.delete(sessionId);
  }, []);

  /**
   * Write binary data to the terminal for a given session.
   * Data is buffered until the terminal is registered AND marked ready
   * (after first fit), so scrollback replays at the correct size.
   */
  const writeBinaryData = useCallback((sessionId: string, data: Uint8Array) => {
    log('writeBinaryData:', sessionId, 'len:', data.length);

    const terminal = terminalsRef.current.get(sessionId);
    if (terminal && readyRef.current.has(sessionId)) {
      writeChunk(terminal, data);
    } else {
      // Buffer until terminal is fit
      log('writeBinaryData: buffering (terminal not ready)');
      if (!pendingDataRef.current.has(sessionId)) {
        pendingDataRef.current.set(sessionId, []);
      }
      pendingDataRef.current.get(sessionId)!.push(data);
    }
  }, []);

  const getTerminal = useCallback((sessionId: string) => {
    return terminalsRef.current.get(sessionId);
  }, []);

  /**
   * Subscribe to resize events for a specific session.
   * Returns an unsubscribe function.
   */
  const onSessionResize = useCallback((sessionId: string, callback: SessionResizeCallback) => {
    if (!resizeListenersRef.current.has(sessionId)) {
      resizeListenersRef.current.set(sessionId, new Set());
    }
    resizeListenersRef.current.get(sessionId)!.add(callback);
    return () => {
      resizeListenersRef.current.get(sessionId)?.delete(callback);
    };
  }, []);

  // ---------------------------------------------------------------------------
  // Binary Handler - route binary terminal data
  // ---------------------------------------------------------------------------

  useEffect(() => {
    const unregister = registerBinaryHandler((sessionId, payload) => {
      writeBinaryData(sessionId, payload);
    });
    return unregister;
  }, [registerBinaryHandler, writeBinaryData]);

  // ---------------------------------------------------------------------------
  // Message Handler - config + session_resize messages
  // ---------------------------------------------------------------------------

  useEffect(() => {
    const unregister = registerMessageHandler((data) => {
      switch (data.type) {
        case 'config': {
          const msg = data as unknown as ConfigMessage;
          applyConfig(msg);
          break;
        }
        case 'session_resize': {
          const sessionId = data.session_id as string;
          const cols = data.cols as number;
          const rows = data.rows as number;
          log('session_resize:', sessionId, cols, rows);
          const listeners = resizeListenersRef.current.get(sessionId);
          if (listeners) {
            for (const cb of listeners) {
              cb(cols, rows);
            }
          }
          break;
        }
        case '__disconnect': {
          setActiveSession(null);
          pendingDataRef.current.clear();
          readyRef.current.clear();
          break;
        }
      }
    });
    return unregister;
  }, [registerMessageHandler, setActiveSession, applyConfig]);

  const value: TerminalContextValue = {
    activeSessionId,
    options,
    setActiveSession,
    applyConfig,
    registerTerminal,
    unregisterTerminal,
    markTerminalReady,
    writeBinaryData,
    getTerminal,
    onSessionResize,
  };

  return (
    <TerminalContext.Provider value={value}>
      {children}
    </TerminalContext.Provider>
  );
}
