/**
 * Tab state management context.
 *
 * Manages the list of iTerm2 sessions (displayed as tabs) with bidirectional sync.
 * Uses sessionId as the unique identifier since multiple sessions can share the same tabId
 * (split panes in iTerm2).
 *
 * - Inbound: tab_list, tab_switch, tab_created, tab_closed from Mac client
 * - Outbound: tab_switch, tab_create, tab_close to Mac client via relay
 */

import {
  createContext,
  useContext,
  useState,
  useCallback,
  useEffect,
  type ReactNode,
} from 'react';
import type { TabInfo, TabListMessage, TabSwitchMessage, TabCreatedMessage, TabClosedMessage } from '../../shared/protocol';
import { useConnection } from './ConnectionContext';
import { useTerminal } from './TerminalContext';

// =============================================================================
// Context Types
// =============================================================================

interface TabsContextValue {
  tabs: TabInfo[];
  activeTabId: string | null;  // Actually stores sessionId for uniqueness
  activeTab: TabInfo | undefined;
  switchTab: (sessionId: string) => void;
  createTab: () => void;
  closeTab: (sessionId: string) => void;
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

export function TabsProvider({ children }: { children: ReactNode }) {
  const [tabs, setTabs] = useState<TabInfo[]>([]);
  // activeTabId now stores sessionId for uniqueness (split panes share tabId)
  const [activeTabId, setActiveTabId] = useState<string | null>(null);

  const { sendMessage, registerMessageHandler } = useConnection();
  const { setActiveSession, clearTerminal } = useTerminal();

  // -- Inbound handlers -------------------------------------------------------

  const handleSetTabs = useCallback((newTabs: TabInfo[]) => {
    console.log('[TabsContext] handleSetTabs received tabs:', newTabs.map(t => ({
      sessionId: t.sessionId,
      title: t.title,
      isActive: t.isActive,
    })));
    setTabs(newTabs);
    // Find the tab marked as active by Mac, or fall back to first tab
    const active = newTabs.find((t) => t.isActive) || newTabs[0];
    console.log('[TabsContext] Selected active tab:', active?.sessionId, active?.title, 'wasExplicitlyActive:', newTabs.some(t => t.isActive));
    if (active) {
      setActiveTabId(active.sessionId);
      setActiveSession(active.sessionId);
    }
  }, [setActiveSession]);

  const handleTabSwitch = useCallback((msg: { tabId: string; sessionId?: string }) => {
    console.log('[TabsContext] handleTabSwitch called with:', msg);
    setTabs((prev) => {
      // If sessionId is provided directly by Mac, use it
      if (msg.sessionId) {
        const tab = prev.find((t) => t.sessionId === msg.sessionId);
        if (tab) {
          console.log('[TabsContext] Using provided sessionId:', msg.sessionId);
          setActiveTabId(tab.sessionId);
          setActiveSession(tab.sessionId);
          return prev.map((t) => ({
            ...t,
            isActive: t.sessionId === msg.sessionId
          }));
        }
      }

      // Fallback: find sessions in the tab by tabId
      const tabSessions = prev.filter((t) => t.tabId === msg.tabId);
      if (tabSessions.length === 0) {
        console.warn('[TabsContext] handleTabSwitch: No sessions found for tabId:', msg.tabId);
        return prev;
      }

      // Use the first session in the tab (or the one already marked isActive)
      const activeSessionInTab = tabSessions.find(t => t.isActive) || tabSessions[0];
      console.log('[TabsContext] Switching to tab', msg.tabId, 'session:', activeSessionInTab.sessionId);

      setActiveTabId(activeSessionInTab.sessionId);
      setActiveSession(activeSessionInTab.sessionId);

      return prev.map((t) => ({
        ...t,
        isActive: t.sessionId === activeSessionInTab.sessionId
      }));
    });
  }, [setActiveSession]);

  const handleTabCreated = useCallback((tab: TabInfo) => {
    setTabs((prev) => {
      if (prev.find((t) => t.sessionId === tab.sessionId)) return prev;
      const updated = tab.isActive
        ? [...prev, tab].map((t) => ({ ...t, isActive: t.sessionId === tab.sessionId }))
        : [...prev, tab];
      return updated;
    });
    if (tab.isActive) {
      setActiveTabId(tab.sessionId);
      setActiveSession(tab.sessionId);
    }
  }, [setActiveSession]);

  const handleTabClosed = useCallback((sessionId: string) => {
    setTabs((prev) => {
      const filtered = prev.filter((t) => t.sessionId !== sessionId);
      return filtered;
    });
    setActiveTabId((prevActiveId) => {
      if (prevActiveId === sessionId) {
        // Need to switch to first remaining tab
        setTabs((prev) => {
          const first = prev[0];
          if (first) {
            setActiveSession(first.sessionId);
            return prev.map((t) => ({ ...t, isActive: t.sessionId === first.sessionId }));
          } else {
            setActiveSession(null);
            return prev;
          }
        });
        return null;
      }
      return prevActiveId;
    });
  }, [setActiveSession]);

  const reset = useCallback(() => {
    setTabs([]);
    setActiveTabId(null);
  }, []);

  // -- Register message handler with connection context -----------------------

  useEffect(() => {
    const unregister = registerMessageHandler((data) => {
      switch (data.type) {
        case 'tab_list': {
          const msg = data as unknown as TabListMessage;
          handleSetTabs(msg.tabs);
          break;
        }
        case 'tab_switch': {
          const msg = data as unknown as TabSwitchMessage;
          handleTabSwitch({ tabId: msg.tabId, sessionId: msg.sessionId });
          break;
        }
        case 'tab_created': {
          const msg = data as unknown as TabCreatedMessage;
          handleTabCreated(msg.tab);
          break;
        }
        case 'tab_closed': {
          const msg = data as unknown as TabClosedMessage;
          handleTabClosed(msg.tabId);
          break;
        }
        case '__disconnect': {
          reset();
          break;
        }
      }
    });
    return unregister;
  }, [registerMessageHandler, handleSetTabs, handleTabSwitch, handleTabCreated, handleTabClosed, reset]);

  // -- Outbound actions -------------------------------------------------------

  const switchTab = useCallback((sessionId: string) => {
    // Find the tab by sessionId
    const tab = tabs.find((t) => t.sessionId === sessionId);
    if (!tab) return;

    // Clear terminal before switching to prepare for new session's content
    clearTerminal();
    setActiveTabId(sessionId);
    setActiveSession(sessionId);
    setTabs((prev) => prev.map((t) => ({ ...t, isActive: t.sessionId === sessionId })));

    // Send sessionId to Mac
    sendMessage({ type: 'tab_switch', tabId: sessionId });
  }, [tabs, sendMessage, setActiveSession, clearTerminal]);

  const createTabAction = useCallback(() => {
    sendMessage({ type: 'tab_create' });
  }, [sendMessage]);

  const closeTabAction = useCallback((sessionId: string) => {
    // Find the tab to get its tabId for the Mac
    const tab = tabs.find((t) => t.sessionId === sessionId);
    sendMessage({ type: 'tab_close', tabId: tab?.tabId || sessionId });
  }, [tabs, sendMessage]);

  const value: TabsContextValue = {
    tabs,
    activeTabId,
    activeTab: tabs.find((t) => t.sessionId === activeTabId),
    switchTab,
    createTab: createTabAction,
    closeTab: closeTabAction,
  };

  return (
    <TabsContext.Provider value={value}>
      {children}
    </TabsContext.Provider>
  );
}
