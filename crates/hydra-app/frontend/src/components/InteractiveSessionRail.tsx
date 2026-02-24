import { useState } from 'react';
import type { CSSProperties } from 'react';
import { Badge, Button } from './design-system';
import type { InteractiveSessionSummary } from '../types';

type SessionLifecycle = 'running' | 'completed' | 'failed' | 'stopped' | 'paused';

interface InteractiveSessionRailProps {
  sessions: InteractiveSessionSummary[];
  selectedSessionId: string | null;
  onSelectSession: (sessionId: string) => void;
  onCreateSession: () => void;
  onStopSession: (sessionId: string) => void;
  creating: boolean;
}

const lifecycleBadgeVariant: Record<SessionLifecycle, 'info' | 'success' | 'danger' | 'warning'> = {
  running: 'info',
  completed: 'success',
  failed: 'danger',
  stopped: 'warning',
  paused: 'warning',
};

const lifecycleLabel: Record<SessionLifecycle, string> = {
  running: 'Running',
  completed: 'Completed',
  failed: 'Failed',
  stopped: 'Stopped',
  paused: 'Paused',
};

function toLifecycle(status: string): SessionLifecycle {
  if (status === 'running' || status === 'completed' || status === 'failed' || status === 'stopped' || status === 'paused') {
    return status;
  }
  return 'stopped';
}

export function InteractiveSessionRail({
  sessions,
  selectedSessionId,
  onSelectSession,
  onCreateSession,
  onStopSession,
  creating,
}: InteractiveSessionRailProps) {
  const containerStyle: CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    gap: 'var(--space-2)',
    minWidth: 240,
    height: '100%',
  };

  return (
    <div style={containerStyle}>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '0 var(--space-2)',
          marginBottom: 'var(--space-1)',
        }}
      >
        <span
          style={{
            fontSize: 'var(--text-xs)',
            fontWeight: 'var(--weight-semibold)' as unknown as number,
            color: 'var(--color-text-muted)',
            textTransform: 'uppercase',
            letterSpacing: '0.05em',
          }}
        >
          Sessions
        </span>
        <Button
          variant="primary"
          size="sm"
          onClick={onCreateSession}
          loading={creating}
          data-testid="create-session-btn"
        >
          + New
        </Button>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 'var(--space-1)' }}>
        {sessions.length === 0 && (
          <div
            style={{
              color: 'var(--color-text-muted)',
              fontSize: 'var(--text-sm)',
              padding: 'var(--space-3) var(--space-2)',
            }}
            data-testid="empty-session-state"
          >
            No sessions yet. Create one to get started.
          </div>
        )}

        {sessions.map((session) => (
          <SessionRailItem
            key={session.sessionId}
            session={session}
            selected={selectedSessionId === session.sessionId}
            onSelect={() => onSelectSession(session.sessionId)}
            onStop={() => onStopSession(session.sessionId)}
          />
        ))}
      </div>
    </div>
  );
}

function SessionRailItem({
  session,
  selected,
  onSelect,
  onStop,
}: {
  session: InteractiveSessionSummary;
  selected: boolean;
  onSelect: () => void;
  onStop: () => void;
}) {
  const [hovered, setHovered] = useState(false);
  const lifecycle = toLifecycle(session.status);
  const isRunning = lifecycle === 'running';

  const itemStyle: CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    gap: 'var(--space-2)',
    padding: 'var(--space-2) var(--space-3)',
    borderRadius: 'var(--radius-md)',
    cursor: 'pointer',
    transition: 'all var(--transition-fast)',
    border: selected
      ? '1px solid var(--color-marine-500)'
      : '1px solid transparent',
    backgroundColor: selected
      ? 'color-mix(in srgb, var(--color-marine-500) 10%, var(--color-surface-800))'
      : hovered
        ? 'var(--color-surface-750)'
        : 'transparent',
  };

  const nameStyle: CSSProperties = {
    fontSize: 'var(--text-sm)',
    fontWeight: selected
      ? ('var(--weight-semibold)' as unknown as number)
      : ('var(--weight-normal)' as unknown as number),
    color: selected ? 'var(--color-text-primary)' : 'var(--color-text-secondary)',
    fontFamily: 'var(--font-mono)',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  };

  const metaStyle: CSSProperties = {
    fontSize: 'var(--text-xs)',
    color: 'var(--color-text-muted)',
    marginTop: 2,
  };

  return (
    <div
      style={itemStyle}
      onClick={onSelect}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      data-testid={`session-item-${session.sessionId}`}
    >
      <div style={{ minWidth: 0, flex: 1 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
          {isRunning && <PulsingDot />}
          <span style={nameStyle}>{session.agentKey}</span>
        </div>
        <div style={metaStyle}>
          {session.eventCount} events · {session.sessionId.slice(0, 8)}
        </div>
      </div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-1)', flexShrink: 0 }}>
        <Badge variant={lifecycleBadgeVariant[lifecycle]} dot>
          {lifecycleLabel[lifecycle]}
        </Badge>
        {isRunning && selected && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onStop();
            }}
            title="Stop session"
            data-testid={`stop-session-${session.sessionId}`}
            style={{
              background: 'none',
              border: 'none',
              color: 'var(--color-danger-400)',
              cursor: 'pointer',
              fontSize: 'var(--text-sm)',
              padding: '2px',
              lineHeight: 1,
            }}
          >
            ■
          </button>
        )}
      </div>
    </div>
  );
}

function PulsingDot() {
  const dotStyle: CSSProperties = {
    width: 8,
    height: 8,
    borderRadius: '50%',
    backgroundColor: 'var(--color-marine-400)',
    flexShrink: 0,
    animation: 'pulse-dot 1.5s ease-in-out infinite',
  };

  return (
    <>
      <style>{`
        @keyframes pulse-dot {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.3; }
        }
      `}</style>
      <span style={dotStyle} />
    </>
  );
}
