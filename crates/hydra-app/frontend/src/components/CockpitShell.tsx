import type { CSSProperties, ReactNode } from 'react';

interface CockpitShellProps {
  leftRail: ReactNode;
  topStrip: ReactNode;
  center: ReactNode;
  rightRail?: ReactNode;
}

export function CockpitShell({ leftRail, topStrip, center, rightRail }: CockpitShellProps) {
  const shellStyle: CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    height: '100vh',
    minWidth: 1024,
    overflow: 'hidden',
  };

  const topStripStyle: CSSProperties = {
    flexShrink: 0,
    borderBottom: '1px solid var(--color-border-700)',
    backgroundColor: 'var(--color-bg-900)',
  };

  const bodyStyle: CSSProperties = {
    display: 'flex',
    flex: 1,
    minHeight: 0,
  };

  const leftRailStyle: CSSProperties = {
    width: 88,
    flexShrink: 0,
    borderRight: '1px solid var(--color-border-700)',
    backgroundColor: 'var(--color-bg-900)',
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    padding: 'var(--space-3) 4px 4px',
    gap: 'var(--space-2)',
    overflowY: 'auto',
  };

  const centerStyle: CSSProperties = {
    flex: 1,
    minWidth: 0,
    minHeight: 0,
    display: 'flex',
    flexDirection: 'column',
    overflow: 'hidden',
  };

  const rightRailStyle: CSSProperties = {
    width: 320,
    flexShrink: 0,
    borderLeft: '1px solid var(--color-border-700)',
    backgroundColor: 'var(--color-bg-850)',
    overflowY: 'auto',
    display: 'flex',
    flexDirection: 'column',
  };

  return (
    <div style={shellStyle} data-testid="cockpit-shell">
      <div style={topStripStyle} data-testid="cockpit-top-strip">
        {topStrip}
      </div>
      <div style={bodyStyle}>
        <div style={leftRailStyle} data-testid="cockpit-left-rail">
          {leftRail}
        </div>
        <div style={centerStyle} data-testid="cockpit-center">
          {center}
        </div>
        {rightRail && (
          <div style={rightRailStyle} data-testid="cockpit-right-rail">
            {rightRail}
          </div>
        )}
      </div>
    </div>
  );
}

interface NavRailButtonProps {
  icon: ReactNode;
  label: string;
  active?: boolean;
  onClick: () => void;
  'data-testid'?: string;
}

export function NavRailButton({
  icon,
  label,
  active = false,
  onClick,
  'data-testid': testId,
}: NavRailButtonProps) {
  const btnStyle: CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 1,
    width: 72,
    height: 60,
    padding: '3px',
    borderRadius: 'var(--radius-md)',
    border: 'none',
    background: active
      ? 'color-mix(in srgb, var(--color-marine-500) 15%, transparent)'
      : 'transparent',
    color: active ? 'var(--color-marine-400)' : 'var(--color-text-muted)',
    cursor: 'pointer',
    transition: 'all var(--transition-fast)',
    fontFamily: 'var(--font-family)',
  };

  const iconStyle: CSSProperties = {
    width: 18,
    height: 18,
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: 0,
  };

  const labelStyle: CSSProperties = {
    fontSize: '8px',
    lineHeight: 1.1,
    letterSpacing: '0.02em',
    textTransform: 'none',
    fontWeight: 'var(--weight-medium)' as unknown as number,
    textAlign: 'center',
    whiteSpace: 'normal',
    overflowWrap: 'anywhere',
    wordBreak: 'break-word',
    maxWidth: '100%',
  };

  return (
    <button
      type="button"
      style={btnStyle}
      onClick={onClick}
      title={label}
      data-testid={testId}
    >
      <span style={iconStyle}>{icon}</span>
      <span style={labelStyle}>{label}</span>
    </button>
  );
}

interface TopStripProps {
  workspacePath: string | null;
  runStatus: string;
  runId: string | null;
  adapterCount: number;
  experimentalCount: number;
}

export function TopStrip({
  workspacePath,
  runStatus,
  runId,
  adapterCount,
  experimentalCount,
}: TopStripProps) {
  const isRunning = runStatus === 'running' || runStatus === 'starting';
  const isDone = runStatus === 'completed' || runStatus === 'failed';

  const containerStyle: CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    height: 104,
    padding: '0 var(--space-4)',
    gap: 'var(--space-4)',
  };

  const leftStyle: CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    gap: 'var(--space-3)',
    minWidth: 0,
    flex: 1,
  };

  const rightStyle: CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    gap: 'var(--space-3)',
    flexShrink: 0,
  };

  const brandStyle: CSSProperties = {
    display: 'inline-flex',
    alignItems: 'center',
    gap: 'var(--space-2)',
    fontSize: 'var(--text-lg)',
    fontWeight: 'var(--weight-bold)' as unknown as number,
    color: 'var(--color-green-400)',
    fontFamily: 'var(--font-mono)',
    flexShrink: 0,
  };

  const brandIconStyle: CSSProperties = {
    width: 96,
    height: 96,
    objectFit: 'contain',
    flexShrink: 0,
    display: 'block',
    filter: 'brightness(1.25) saturate(1.2) drop-shadow(0 0 8px rgba(33, 255, 196, 0.35))',
  };

  const workspaceStyle: CSSProperties = {
    fontSize: 'var(--text-xs)',
    color: 'var(--color-text-muted)',
    fontFamily: 'var(--font-mono)',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  };

  const statusDotStyle: CSSProperties = {
    width: 8,
    height: 8,
    borderRadius: '50%',
    flexShrink: 0,
    backgroundColor: isRunning
      ? 'var(--color-marine-400)'
      : isDone
        ? runStatus === 'completed'
          ? 'var(--color-green-500)'
          : 'var(--color-danger-500)'
        : 'var(--color-text-muted)',
    ...(isRunning ? { animation: 'pulse-dot 1.5s ease-in-out infinite' } : {}),
  };

  const statusTextStyle: CSSProperties = {
    fontSize: 'var(--text-xs)',
    color: 'var(--color-text-secondary)',
  };

  const badgeStyle: CSSProperties = {
    fontSize: 'var(--text-xs)',
    padding: '2px var(--space-2)',
    borderRadius: 'var(--radius-full)',
    backgroundColor: 'color-mix(in srgb, var(--color-marine-500) 12%, transparent)',
    color: 'var(--color-marine-400)',
  };

  const versionStyle: CSSProperties = {
    fontSize: 'var(--text-xs)',
    color: 'var(--color-text-muted)',
  };

  return (
    <div style={containerStyle} data-testid="top-strip-content">
      <style>{`@keyframes pulse-dot { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }`}</style>
      <div style={leftStyle}>
        <span style={brandStyle}>
          <img src="/hydra-icon.png?v=1" alt="" aria-hidden="true" style={brandIconStyle} />
          <span>Hydra</span>
        </span>
        <span style={workspaceStyle} data-testid="strip-workspace">
          {workspacePath ?? '(current repo)'}
        </span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
          <span style={statusDotStyle} />
          <span style={statusTextStyle}>{runStatus}</span>
        </div>
        {runId && (
          <span style={badgeStyle}>
            {runId.slice(0, 8)}
          </span>
        )}
      </div>
      <div style={rightStyle}>
        <span style={badgeStyle}>{adapterCount} adapters</span>
        {experimentalCount > 0 && (
          <span
            style={{
              ...badgeStyle,
              backgroundColor: 'color-mix(in srgb, var(--color-warning-500) 12%, transparent)',
              color: 'var(--color-warning-400)',
            }}
          >
            {experimentalCount} experimental
          </span>
        )}
        <span style={versionStyle}>v0.1.0-alpha</span>
      </div>
    </div>
  );
}
