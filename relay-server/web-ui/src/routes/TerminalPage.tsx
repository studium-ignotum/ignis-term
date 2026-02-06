import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useConnection } from '../lib/context/ConnectionContext';
import { useTerminal } from '../lib/context/TerminalContext';
import { useTabs } from '../lib/context/TabsContext';
import Terminal from '../lib/components/Terminal';
import TerminalTabs from '../lib/components/TerminalTabs';
import MobileControlBar from '../lib/components/MobileControlBar';
import ConnectionStatus from '../lib/components/ConnectionStatus';
import './TerminalPage.css';

export default function TerminalPage() {
  const navigate = useNavigate();
  const { state, isConnected, disconnect, sendTerminalInput, sendTerminalResize } = useConnection();
  const { activeSessionId, options } = useTerminal();
  const { tabs } = useTabs();

  // Redirect to login if disconnected
  useEffect(() => {
    if (state === 'disconnected') {
      navigate('/login');
    }
  }, [state, navigate]);

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
