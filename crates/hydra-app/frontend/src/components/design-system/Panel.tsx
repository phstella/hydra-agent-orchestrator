import type { CSSProperties, ReactNode } from 'react';

interface PanelProps {
  title?: string;
  children: ReactNode;
  headerRight?: ReactNode;
  style?: CSSProperties;
}

export function Panel({ title, children, headerRight, style }: PanelProps) {
  const containerStyle: CSSProperties = {
    backgroundColor: 'var(--color-surface-800)',
    border: '1px solid var(--color-border-700)',
    borderRadius: 'var(--radius-lg)',
    overflow: 'hidden',
    ...style,
  };

  const headerStyle: CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: 'var(--space-3) var(--space-4)',
    borderBottom: '1px solid var(--color-border-700)',
    backgroundColor: 'var(--color-bg-900)',
  };

  const titleStyle: CSSProperties = {
    fontSize: 'var(--text-sm)',
    fontWeight: 'var(--weight-semibold)' as unknown as number,
    color: 'var(--color-text-secondary)',
    textTransform: 'uppercase',
    letterSpacing: '0.05em',
  };

  const bodyStyle: CSSProperties = {
    padding: 'var(--space-4)',
  };

  return (
    <div style={containerStyle}>
      {title && (
        <div style={headerStyle}>
          <span style={titleStyle}>{title}</span>
          {headerRight}
        </div>
      )}
      <div style={bodyStyle}>{children}</div>
    </div>
  );
}
