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
 * - Per-session data filtering (only show active session)
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

interface TerminalContextValue {
  activeSessionId: string | null;
  options: ITerminalOptions;
  setActiveSession: (sessionId: string | null) => void;
  applyConfig: (config: ConfigMessage) => void;
  registerTerminal: (sessionId: string, terminal: Terminal) => void;
  unregisterTerminal: (sessionId: string) => void;
  /** Write binary data to terminal (used internally by binary handler) */
  writeBinaryData: (sessionId: string, data: Uint8Array) => void;
  getTerminal: (sessionId: string) => Terminal | undefined;
  clearTerminal: () => void;
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
  // Buffer binary data that arrives before terminal mounts (keyed by sessionId)
  const pendingDataRef = useRef<Map<string, Uint8Array[]>>(new Map());
  // Ref for immediate sync access to active session
  const activeSessionIdRef = useRef<string | null>(null);

  const { registerMessageHandler, registerBinaryHandler } = useConnection();

  const setActiveSession = useCallback((sessionId: string | null) => {
    activeSessionIdRef.current = sessionId;
    setActiveSessionId(sessionId);
  }, []);

  const applyConfig = useCallback((config: ConfigMessage) => {
    log('applyConfig:', config);
    setOptions(configToXtermOptions(config));
  }, []);

  const findTerminal = useCallback((): Terminal | undefined => {
    // Only one Terminal component renders at a time (for active session)
    const entries = terminalsRef.current.values();
    const first = entries.next();
    return first.done ? undefined : first.value;
  }, []);

  /**
   * Register a terminal instance. Flush any pending data for this session.
   */
  const registerTerminal = useCallback((sessionId: string, terminal: Terminal) => {
    log('registerTerminal:', sessionId);
    terminalsRef.current.set(sessionId, terminal);

    // Flush pending data for this session
    const pending = pendingDataRef.current.get(sessionId);
    if (pending && pending.length > 0) {
      log('registerTerminal: flushing', pending.length, 'buffered chunks');
      for (const chunk of pending) {
        // Use writeUtf8 for efficient binary writes
        (terminal as unknown as { writeUtf8: (data: Uint8Array) => void }).writeUtf8?.(chunk)
          ?? terminal.write(new TextDecoder().decode(chunk));
      }
      pendingDataRef.current.delete(sessionId);
    }
  }, []);

  const unregisterTerminal = useCallback((sessionId: string) => {
    terminalsRef.current.delete(sessionId);
  }, []);

  /**
   * Write binary data to the terminal for a given session.
   * Uses writeUtf8 for efficiency when available.
   */
  const writeBinaryData = useCallback((sessionId: string, data: Uint8Array) => {
    log('writeBinaryData:', sessionId, 'len:', data.length);

    // Filter: only write data for the active session
    if (activeSessionIdRef.current && sessionId !== activeSessionIdRef.current) {
      log('writeBinaryData: filtered (not active session)');
      return;
    }

    const terminal = findTerminal();
    if (terminal) {
      // Use writeUtf8 for binary efficiency (xterm.js 5.x+)
      const termWithUtf8 = terminal as unknown as { writeUtf8?: (data: Uint8Array) => void };
      if (termWithUtf8.writeUtf8) {
        termWithUtf8.writeUtf8(data);
      } else {
        // Fallback to string write
        terminal.write(new TextDecoder().decode(data));
      }
    } else {
      // Buffer data until terminal mounts
      log('writeBinaryData: buffering (no terminal yet)');
      if (!pendingDataRef.current.has(sessionId)) {
        pendingDataRef.current.set(sessionId, []);
      }
      pendingDataRef.current.get(sessionId)!.push(data);
    }
  }, [findTerminal]);

  const getTerminal = useCallback((sessionId: string) => {
    return terminalsRef.current.get(sessionId);
  }, []);

  const clearTerminal = useCallback(() => {
    const terminal = findTerminal();
    if (terminal) {
      terminal.reset();
    }
    pendingDataRef.current.clear();
  }, [findTerminal]);

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
  // Message Handler - config messages
  // ---------------------------------------------------------------------------

  useEffect(() => {
    const unregister = registerMessageHandler((data) => {
      switch (data.type) {
        case 'config': {
          const msg = data as unknown as ConfigMessage;
          applyConfig(msg);
          break;
        }
        case '__disconnect': {
          setActiveSession(null);
          pendingDataRef.current.clear();
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
    writeBinaryData,
    getTerminal,
    clearTerminal,
  };

  return (
    <TerminalContext.Provider value={value}>
      {children}
    </TerminalContext.Provider>
  );
}
