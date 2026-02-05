import { useEffect, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { useConnection } from '../lib/context/ConnectionContext';
import { useTerminal } from '../lib/context/TerminalContext';
import { useTabs } from '../lib/context/TabsContext';
import Terminal from '../lib/components/Terminal';
import TerminalTabs from '../lib/components/TerminalTabs';
import MobileControlBar from '../lib/components/MobileControlBar';
import ConnectionStatus from '../lib/components/ConnectionStatus';
import './TerminalPage.css';

// Debounce delay before requesting screen refresh after resize (ms)
const SCREEN_REFRESH_DELAY_MS = 300;

export default function TerminalPage() {
  const navigate = useNavigate();
  const { state, isConnected, disconnect, sendTerminalInput, sendTerminalResize, sendScreenRefresh } = useConnection();
  const { activeSessionId, options } = useTerminal();
  const { tabs } = useTabs();

  // Track previous dimensions to detect significant size changes
  const prevDimsRef = useRef<{ cols: number; rows: number } | null>(null);
  const refreshTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Redirect to login if disconnected
  useEffect(() => {
    if (state === 'disconnected') {
      navigate('/login');
    }
  }, [state, navigate]);

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (refreshTimeoutRef.current) {
        clearTimeout(refreshTimeoutRef.current);
      }
    };
  }, []);

  function handleDisconnect() {
    disconnect();
  }

  function handleInput(data: string) {
    if (activeSessionId) {
      sendTerminalInput(activeSessionId, data);
    }
  }

  function handleBinaryInput(data: string) {
    if (activeSessionId) {
      sendTerminalInput(activeSessionId, data);
    }
  }

  function handleResize(cols: number, rows: number) {
    if (activeSessionId) {
      sendTerminalResize(activeSessionId, cols, rows);

      // Check if this is a significant resize (e.g., mobile orientation change)
      const prev = prevDimsRef.current;
      const isSignificantResize = prev && (
        Math.abs(cols - prev.cols) > 5 || Math.abs(rows - prev.rows) > 5
      );

      prevDimsRef.current = { cols, rows };

      // Request screen refresh after resize settles (debounced)
      // This ensures the Mac sends reformatted content for the new size
      if (refreshTimeoutRef.current) {
        clearTimeout(refreshTimeoutRef.current);
      }

      // Only request refresh for significant resizes to avoid spam
      if (isSignificantResize || !prev) {
        refreshTimeoutRef.current = setTimeout(() => {
          if (activeSessionId) {
            sendScreenRefresh(activeSessionId);
          }
        }, SCREEN_REFRESH_DELAY_MS);
      }
    }
  }

  function handleMobileKey(data: string) {
    if (activeSessionId) {
      sendTerminalInput(activeSessionId, data);
    }
  }

  const hasTabs = tabs.length > 0;

  return (
    <div className="terminal-page">
      <header className="header-bar">
        <ConnectionStatus />
        <div className="header-spacer" />
        <button className="btn-disconnect" onClick={handleDisconnect}>
          Disconnect
        </button>
      </header>

      {isConnected && activeSessionId ? (
        <div className="main-layout">
          {hasTabs && <TerminalTabs />}
          <div className="terminal-column">
            <div className="terminal-area">
              <Terminal
                sessionId={activeSessionId}
                options={options}
                onInput={handleInput}
                onBinaryInput={handleBinaryInput}
                onTerminalResize={handleResize}
              />
            </div>
            <MobileControlBar onKey={handleMobileKey} />
          </div>
        </div>
      ) : (
        <main className="waiting-state">
          <div className="waiting-content">
            <div className="spinner" />
            <h2>Waiting for terminal session...</h2>
            <p className="waiting-detail">
              The Mac client will send terminal data once connected.
            </p>
            <div className="status-info">
              <span className="status-badge">{state}</span>
            </div>
          </div>
        </main>
      )}
    </div>
  );
}
