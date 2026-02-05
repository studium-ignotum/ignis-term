/**
 * Terminal state management context.
 *
 * Manages the active terminal session, xterm.js options, and a registry of
 * Terminal instances for routing incoming terminal_data messages to the correct
 * terminal. Also handles iTerm2 config messages by converting them to xterm.js
 * options via the iterm-theme module.
 *
 * Data that arrives before a terminal is mounted is buffered and flushed once
 * a terminal registers.
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

// Debug logging - set to true to trace data flow
const DEBUG = true;
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
  writeData: (sessionId: string, data: string) => void;
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
  // Buffer for data that arrives before a terminal is mounted (with session ID for correct routing)
  const pendingDataRef = useRef<{sessionId: string; data: string}[]>([]);
  // Ref for immediate sync access to active session (avoids race with async state)
  const activeSessionIdRef = useRef<string | null>(null);

  const { registerMessageHandler } = useConnection();

  const setActiveSession = useCallback((sessionId: string | null) => {
    activeSessionIdRef.current = sessionId;  // Sync update
    setActiveSessionId(sessionId);  // Async state update for UI
  }, []);

  const applyConfig = useCallback((config: ConfigMessage) => {
    log('applyConfig:', config);
    setOptions(configToXtermOptions(config));
  }, []);

  const findTerminal = useCallback((): Terminal | undefined => {
    // Only one Terminal component renders at a time, so return whatever is registered
    const entries = terminalsRef.current.values();
    const first = entries.next();
    return first.done ? undefined : first.value;
  }, []);

  const registerTerminal = useCallback((sessionId: string, terminal: Terminal) => {
    log('registerTerminal:', sessionId, 'pending chunks:', pendingDataRef.current.length);
    terminalsRef.current.set(sessionId, terminal);
    // Only flush data that matches THIS session (prevents cross-session data contamination)
    const matchingData = pendingDataRef.current.filter(d => d.sessionId === sessionId);
    if (matchingData.length > 0) {
      log('registerTerminal: flushing', matchingData.length, 'matching chunks (filtered from', pendingDataRef.current.length, 'total)');
      for (const chunk of matchingData) {
        terminal.write(chunk.data);
      }
      // Remove only the flushed data, keep data for other sessions
      pendingDataRef.current = pendingDataRef.current.filter(d => d.sessionId !== sessionId);
    }
  }, []);

  const unregisterTerminal = useCallback((sessionId: string) => {
    terminalsRef.current.delete(sessionId);
  }, []);

  const writeData = useCallback((sessionId: string, data: string) => {
    // Filter data to only show for the active session
    // Exception: if activeSession is not yet set, buffer the data (it will be flushed when terminal registers)
    log('writeData called:', {
      sessionId,
      dataLen: data.length,
      activeSession: activeSessionIdRef.current,
      terminalCount: terminalsRef.current.size,
    });

    // If we have an active session and this data is for a different session, skip it
    if (activeSessionIdRef.current && sessionId !== activeSessionIdRef.current) {
      log('writeData: FILTERED OUT (wrong session)');
      return;
    }

    const terminal = findTerminal();
    if (terminal) {
      log('writeData: writing to terminal');
      terminal.write(data);
    } else {
      // Terminal not mounted yet — buffer WITH session ID so we can route correctly on register
      log('writeData: buffering (no terminal yet), sessionId:', sessionId, 'buffer size:', pendingDataRef.current.length + 1);
      pendingDataRef.current.push({ sessionId, data });
    }
  }, [findTerminal]);

  const getTerminal = useCallback((sessionId: string) => {
    return terminalsRef.current.get(sessionId);
  }, []);

  const clearTerminal = useCallback(() => {
    const terminal = findTerminal();
    if (terminal) {
      // Use reset() for full terminal reset (clears screen + scrollback + resets state)
      terminal.reset();
    }
    pendingDataRef.current = [];
  }, [findTerminal]);

  // Register message handler with connection context
  useEffect(() => {
    const unregister = registerMessageHandler((data) => {
      log('message received:', data.type);
      switch (data.type) {
        // Note: 'joined' message contains relay sessionId, not iTerm2 sessionId
        // Active session is set by TabsContext when tab_list is received
        case 'terminal_data':
        case 'initial_terminal_data': {
          const msg = data as unknown as { sessionId: string; payload: string };
          log('terminal data for session:', msg.sessionId, 'length:', msg.payload?.length);
          writeData(msg.sessionId, msg.payload);
          break;
        }
        case 'config': {
          const msg = data as unknown as ConfigMessage;
          log('config message received');
          applyConfig(msg);
          break;
        }
        case '__disconnect': {
          setActiveSession(null);
          pendingDataRef.current = [];
          break;
        }
      }
    });
    return unregister;
  }, [registerMessageHandler, setActiveSession, writeData, applyConfig]);

  const value: TerminalContextValue = {
    activeSessionId,
    options,
    setActiveSession,
    applyConfig,
    registerTerminal,
    unregisterTerminal,
    writeData,
    getTerminal,
    clearTerminal,
  };

  return (
    <TerminalContext.Provider value={value}>
      {children}
    </TerminalContext.Provider>
  );
}
