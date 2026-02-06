/**
 * Session/Tab state management context.
 *
 * Manages shell sessions displayed as tabs in the UI. Sessions are discovered
 * from binary frame headers (when terminal data arrives) or from explicit
 * session_connected/session_disconnected JSON messages.
 *
 * Key behaviors:
 * - Sessions are added when first binary frame arrives for a new sessionId
 * - Auto-switch to first session (per phase context decision)
 * - Sessions marked as disconnected after session_disconnected message
 * - Disconnected sessions removed after 5 seconds
 */

import {
  createContext,
  useContext,
  useState,
  useCallback,
  useEffect,
  useRef,
  type ReactNode,
} from 'react';
import type { SessionConnectedMessage, SessionDisconnectedMessage } from '../../shared/protocol';
import { useConnection } from './ConnectionContext';
import { useTerminal } from './TerminalContext';

// =============================================================================
// Session Types
// =============================================================================

export interface SessionInfo {
  id: string;
  name: string;
  connected: boolean;
  lastActivity: number; // timestamp
}

// =============================================================================
// Context Types
// =============================================================================

interface TabsContextValue {
  sessions: SessionInfo[];
  activeSessionId: string | null;
  activeSession: SessionInfo | undefined;
  switchSession: (sessionId: string) => void;
  /** Create new tab - currently no-op (shell sessions managed by user) */
  createTab: () => void;
  /** Close tab - currently no-op (shell sessions managed by user) */
  closeTab: (sessionId: string) => void;
  // Legacy aliases for compatibility
  tabs: SessionInfo[];
  activeTabId: string | null;
  activeTab: SessionInfo | undefined;
  switchTab: (sessionId: string) => void;
}

const TabsContext = createContext<TabsContextValue | null>(null);

export function useTabs(): TabsContextValue {
  const ctx = useContext(TabsContext);
  if (!ctx) throw new Error('useTabs must be used within TabsProvider');
  return ctx;
}

// =============================================================================
// Provider
// =============================================================================

// Remove disconnected sessions after this delay (ms)
const DISCONNECTED_REMOVAL_DELAY_MS = 5000;

export function TabsProvider({ children }: { children: ReactNode }) {
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);

  // Refs for immediate sync access (avoids stale closures)
  const activeSessionIdRef = useRef<string | null>(null);
  const sessionsRef = useRef<SessionInfo[]>([]);
  const removalTimersRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  const { registerMessageHandler, registerBinaryHandler } = useConnection();
  const { setActiveSession, clearTerminal } = useTerminal();

  // Keep refs in sync
  useEffect(() => {
    activeSessionIdRef.current = activeSessionId;
  }, [activeSessionId]);

  useEffect(() => {
    sessionsRef.current = sessions;
  }, [sessions]);

  // ---------------------------------------------------------------------------
  // Session Management
  // ---------------------------------------------------------------------------

  /**
   * Add or update a session from binary data or session_connected message.
   * If this is the first session, auto-switch to it.
   */
  const addOrUpdateSession = useCallback((sessionId: string, name?: string) => {
    setSessions((prev) => {
      const existing = prev.find((s) => s.id === sessionId);

      if (existing) {
        // Update lastActivity and ensure connected = true
        return prev.map((s) =>
          s.id === sessionId
            ? { ...s, connected: true, lastActivity: Date.now(), name: name ?? s.name }
            : s
        );
      }

      // New session - add it
      const newSession: SessionInfo = {
        id: sessionId,
        name: name ?? sessionId, // Use sessionId as fallback name
        connected: true,
        lastActivity: Date.now(),
      };

      const updated = [...prev, newSession];

      // Auto-switch to first session
      if (prev.length === 0) {
        // Use timeout to avoid state update during render
        setTimeout(() => {
          setActiveSessionId(sessionId);
          activeSessionIdRef.current = sessionId;
          setActiveSession(sessionId);
        }, 0);
      }

      return updated;
    });

    // Clear any pending removal timer for this session
    const timer = removalTimersRef.current.get(sessionId);
    if (timer) {
      clearTimeout(timer);
      removalTimersRef.current.delete(sessionId);
    }
  }, [setActiveSession]);

  /**
   * Mark a session as disconnected. It will be removed after a delay.
   */
  const markSessionDisconnected = useCallback((sessionId: string) => {
    setSessions((prev) =>
      prev.map((s) =>
        s.id === sessionId ? { ...s, connected: false } : s
      )
    );

    // Schedule removal after delay
    const timer = setTimeout(() => {
      setSessions((prev) => {
        const filtered = prev.filter((s) => s.id !== sessionId);

        // If we removed the active session, switch to first remaining
        if (activeSessionIdRef.current === sessionId) {
          const first = filtered[0];
          if (first) {
            setActiveSessionId(first.id);
            activeSessionIdRef.current = first.id;
            setActiveSession(first.id);
          } else {
            setActiveSessionId(null);
            activeSessionIdRef.current = null;
            setActiveSession(null);
          }
        }

        return filtered;
      });
      removalTimersRef.current.delete(sessionId);
    }, DISCONNECTED_REMOVAL_DELAY_MS);

    removalTimersRef.current.set(sessionId, timer);
  }, [setActiveSession]);

  /**
   * Reset all sessions (on disconnect).
   */
  const reset = useCallback(() => {
    setSessions([]);
    setActiveSessionId(null);
    activeSessionIdRef.current = null;

    // Clear all removal timers
    for (const timer of removalTimersRef.current.values()) {
      clearTimeout(timer);
    }
    removalTimersRef.current.clear();
  }, []);

  // ---------------------------------------------------------------------------
  // Binary Handler - Session discovery from terminal data
  // ---------------------------------------------------------------------------

  useEffect(() => {
    const unregister = registerBinaryHandler((sessionId, _data) => {
      // When binary data arrives, ensure session exists
      addOrUpdateSession(sessionId);
    });
    return unregister;
  }, [registerBinaryHandler, addOrUpdateSession]);

  // ---------------------------------------------------------------------------
  // Message Handler - session_connected, session_disconnected
  // ---------------------------------------------------------------------------

  useEffect(() => {
    const unregister = registerMessageHandler((data) => {
      switch (data.type) {
        case 'session_connected': {
          const msg = data as unknown as SessionConnectedMessage;
          addOrUpdateSession(msg.session_id, msg.name);
          break;
        }
        case 'session_disconnected': {
          const msg = data as unknown as SessionDisconnectedMessage;
          markSessionDisconnected(msg.session_id);
          break;
        }
        case '__disconnect': {
          reset();
          break;
        }
      }
    });
    return unregister;
  }, [registerMessageHandler, addOrUpdateSession, markSessionDisconnected, reset]);

  // ---------------------------------------------------------------------------
  // Cleanup timers on unmount
  // ---------------------------------------------------------------------------

  useEffect(() => {
    return () => {
      for (const timer of removalTimersRef.current.values()) {
        clearTimeout(timer);
      }
    };
  }, []);

  // ---------------------------------------------------------------------------
  // Actions
  // ---------------------------------------------------------------------------

  const switchSession = useCallback((sessionId: string) => {
    const session = sessionsRef.current.find((s) => s.id === sessionId);
    if (!session) return;

    // Clear terminal before switch
    clearTerminal();

    setActiveSessionId(sessionId);
    activeSessionIdRef.current = sessionId;
    setActiveSession(sessionId);
  }, [setActiveSession, clearTerminal]);

  // No-op for shell sessions (user manages via shell)
  const createTabAction = useCallback(() => {
    // Shell sessions are created by the user opening new terminal tabs
    console.log('[TabsContext] createTab is a no-op in v2 (shell sessions managed by user)');
  }, []);

  const closeTabAction = useCallback((_sessionId: string) => {
    // Shell sessions are closed by the user closing terminal tabs
    console.log('[TabsContext] closeTab is a no-op in v2 (shell sessions managed by user)');
  }, []);

  // ---------------------------------------------------------------------------
  // Context Value
  // ---------------------------------------------------------------------------

  const value: TabsContextValue = {
    sessions,
    activeSessionId,
    activeSession: sessions.find((s) => s.id === activeSessionId),
    switchSession,
    createTab: createTabAction,
    closeTab: closeTabAction,
    // Legacy aliases
    tabs: sessions,
    activeTabId: activeSessionId,
    activeTab: sessions.find((s) => s.id === activeSessionId),
    switchTab: switchSession,
  };

  return (
    <TabsContext.Provider value={value}>
      {children}
    </TabsContext.Provider>
  );
}
