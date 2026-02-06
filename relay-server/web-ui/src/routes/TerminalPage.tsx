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
  const { state, isConnected, disconnect, sendTerminalInput } = useConnection();
  const { activeSessionId, options } = useTerminal();
  const { tabs, createTab } = useTabs();

  // Redirect to login if disconnected
  useEffect(() => {
    if (state === 'disconnected') {
      navigate('/login');
    }
  }, [state, navigate]);

  function handleDisconnect() {
    disconnect();
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

      {isConnected && hasTabs ? (
        <div className="main-layout">
          {hasTabs && <TerminalTabs />}
          <div className="terminal-column">
            {tabs.map(tab => (
              <div
                key={tab.id}
                className={`terminal-area ${tab.id === activeSessionId ? '' : 'terminal-area-hidden'}`}
              >
                <Terminal
                  sessionId={tab.id}
                  options={options}
                  onInput={(data) => sendTerminalInput(tab.id, data)}
                  onBinaryInput={(data) => sendTerminalInput(tab.id, data)}
                />
              </div>
            ))}
            <MobileControlBar onKey={handleMobileKey} />
          </div>
        </div>
      ) : (
        <main className="waiting-state">
          <div className="waiting-content">
            <h2>No terminal sessions</h2>
            <p className="waiting-detail">
              Create a new session to get started.
            </p>
            <button className="btn-create-session" onClick={createTab}>
              + New Session
            </button>
          </div>
        </main>
      )}
    </div>
  );
}
