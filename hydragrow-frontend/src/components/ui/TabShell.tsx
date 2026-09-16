import { useState, type ReactNode } from 'react';

export interface TabShellTab {
  id: string;
  label: string;
  content: ReactNode;
}

interface TabShellProps {
  title: string;
  subtitle?: string;
  action?: ReactNode;
  tabs: TabShellTab[];
  defaultTabId: string;
  onTabChange?: (tabId: string) => void;
  activeTabId?: string;
}

export const TabShell = ({ title, subtitle, action, tabs, defaultTabId, onTabChange, activeTabId: controlledTabId }: TabShellProps) => {
  const [localTabId, setLocalTabId] = useState(defaultTabId);
  const activeTabId = controlledTabId ?? localTabId;
  const activeTab = tabs.find((tab) => tab.id === activeTabId) ?? tabs[0];

  const handleSelect = (tabId: string) => {
    setLocalTabId(tabId);
    onTabChange?.(tabId);
  };

  return (
    <div className="app-page">
      <div className="page-header">
        <div>
          <h1 className="page-header-title">{title}</h1>
          {subtitle && <p className="page-header-subtitle">{subtitle}</p>}
        </div>
        {action}
      </div>

      <div className="ui-tabbar" role="tablist" aria-label={title}>
        {tabs.map((tab) => (
          <button
            key={tab.id}
            role="tab"
            type="button"
            aria-selected={tab.id === activeTabId}
            className={`ui-tab ${tab.id === activeTabId ? 'ui-tab-active' : ''}`}
            onClick={() => handleSelect(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </div>

      <div role="tabpanel">{activeTab?.content}</div>
    </div>
  );
};
