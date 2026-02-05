import { useTabs } from '../context/TabsContext';
import './TerminalTabs.css';

export default function TerminalTabs() {
  const { tabs, activeTabId, switchTab, createTab, closeTab } = useTabs();

  function handleCloseTab(event: React.MouseEvent, sessionId: string) {
    event.stopPropagation();
    closeTab(sessionId);
  }

  return (
    <aside className="tab-sidebar">
      <div className="tab-header">
        <span className="tab-header-label">Tabs</span>
        <button
          className="btn-new-tab"
          onClick={createTab}
          title="New tab"
          aria-label="Create new tab"
        >
          +
        </button>
      </div>

      <div className="tab-list" role="tablist" aria-label="Terminal tabs">
        {tabs.map((tab) => (
          <div
            key={tab.sessionId}
            className={`tab-item${tab.sessionId === activeTabId ? ' active' : ''}`}
            role="tab"
            tabIndex={0}
            aria-selected={tab.tabId === activeTabId}
            onClick={() => switchTab(tab.sessionId)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') switchTab(tab.sessionId);
            }}
            title={tab.title}
          >
            <span className="tab-title">{tab.title || 'Terminal'}</span>
            <button
              className="btn-close-tab"
              onClick={(e) => handleCloseTab(e, tab.sessionId)}
              title="Close tab"
              aria-label={`Close tab ${tab.title}`}
            >
              &times;
            </button>
          </div>
        ))}
      </div>

      {tabs.length === 0 && <div className="tab-empty">No tabs</div>}
    </aside>
  );
}
