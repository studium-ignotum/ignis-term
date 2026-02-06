import { useTabs } from '../context/TabsContext';
import './TerminalTabs.css';

export default function TerminalTabs() {
  const { sessions, activeSessionId, switchSession, createTab, closeTab } = useTabs();

  function handleCloseSession(event: React.MouseEvent, sessionId: string) {
    event.stopPropagation();
    closeTab(sessionId);
  }

  return (
    <aside className="tab-sidebar">
      <div className="tab-header">
        <span className="tab-header-label">Sessions</span>
        <button
          className="btn-new-tab"
          onClick={createTab}
          title="New session"
          aria-label="Create new session"
        >
          +
        </button>
      </div>

      <div className="tab-list" role="tablist" aria-label="Terminal sessions">
        {sessions.map((session) => (
          <div
            key={session.id}
            className={`tab-item${session.id === activeSessionId ? ' active' : ''}${!session.connected ? ' disconnected' : ''}`}
            role="tab"
            tabIndex={0}
            aria-selected={session.id === activeSessionId}
            onClick={() => switchSession(session.id)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') switchSession(session.id);
            }}
            title={session.name}
          >
            <span className="tab-title">{session.name || 'Terminal'}</span>
            {!session.connected && <span className="disconnected-badge">offline</span>}
            <button
              className="btn-close-tab"
              onClick={(e) => handleCloseSession(e, session.id)}
              title="Close session"
              aria-label={`Close session ${session.name}`}
            >
              &times;
            </button>
          </div>
        ))}
      </div>

      {sessions.length === 0 && <div className="tab-empty">No sessions</div>}
    </aside>
  );
}
