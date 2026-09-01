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
}

export const TabShell = ({ title, subtitle, action, tabs, defaultTabId, onTabChange }: TabShellProps) => {
  const [activeTabId, setActiveTabId] = useState(defaultTabId);
  const activeTab = tabs.find((tab) => tab.id === activeTabId) ?? tabs[0];

  const handleSelect = (tabId: string) => {
    setActiveTabId(tabId);
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
