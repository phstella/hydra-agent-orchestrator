import { useState } from 'react';
import type { CSSProperties, ReactNode } from 'react';

interface Tab {
  id: string;
  label: string;
  badge?: string;
}

interface TabsProps {
  tabs: Tab[];
  activeTab: string;
  onTabChange: (id: string) => void;
  children: ReactNode;
}

export function Tabs({ tabs, activeTab, onTabChange, children }: TabsProps) {
  const containerStyle: CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
  };

  const tabListStyle: CSSProperties = {
    display: 'flex',
    gap: 'var(--space-1)',
    borderBottom: '1px solid var(--color-border-700)',
    padding: '0 var(--space-4)',
  };

  return (
    <div style={containerStyle}>
      <div style={tabListStyle} role="tablist">
        {tabs.map((tab) => (
          <TabButton
            key={tab.id}
            tab={tab}
            active={activeTab === tab.id}
            onClick={() => onTabChange(tab.id)}
          />
        ))}
      </div>
      <div role="tabpanel">{children}</div>
    </div>
  );
}

function TabButton({ tab, active, onClick }: { tab: Tab; active: boolean; onClick: () => void }) {
  const [hovered, setHovered] = useState(false);

  const style: CSSProperties = {
    padding: 'var(--space-2) var(--space-4)',
    fontSize: 'var(--text-sm)',
    fontWeight: active ? ('var(--weight-semibold)' as unknown as number) : ('var(--weight-normal)' as unknown as number),
    color: active ? 'var(--color-marine-400)' : 'var(--color-text-muted)',
    background: 'transparent',
    border: 'none',
    borderBottom: active ? '2px solid var(--color-marine-500)' : '2px solid transparent',
    cursor: 'pointer',
    transition: 'all var(--transition-fast)',
    display: 'flex',
    alignItems: 'center',
    gap: 'var(--space-2)',
    fontFamily: 'var(--font-family)',
    ...(hovered && !active ? { color: 'var(--color-text-secondary)' } : {}),
  };

  const badgeStyle: CSSProperties = {
    fontSize: 'var(--text-xs)',
    padding: '0 6px',
    borderRadius: 'var(--radius-full)',
    backgroundColor: active ? 'rgba(47, 111, 159, 0.2)' : 'rgba(107, 143, 128, 0.15)',
    color: active ? 'var(--color-marine-400)' : 'var(--color-text-muted)',
  };

  return (
    <button
      role="tab"
      aria-selected={active}
      style={style}
      onClick={onClick}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      {tab.label}
      {tab.badge && <span style={badgeStyle}>{tab.badge}</span>}
    </button>
  );
}
