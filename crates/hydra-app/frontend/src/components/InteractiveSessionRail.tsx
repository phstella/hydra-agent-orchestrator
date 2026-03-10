import { useState, useMemo } from 'react';
import type { CSSProperties } from 'react';
import { Badge } from './design-system';
import type { InteractiveSessionSummary } from '../types';

type SessionLifecycle = 'running' | 'completed' | 'failed' | 'stopped' | 'paused';

interface InteractiveSessionRailProps {
  sessions: InteractiveSessionSummary[];
  selectedSessionId: string | null;
  pollErrors: Map<string, string>;
  reduceMotion?: boolean;
  onSelectSession: (sessionId: string) => void;
  onStopSession: (sessionId: string) => void;
  onRemoveSession: (sessionId: string) => void;
  onClearStoppedSessions: () => void;
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

/** Compute elapsed time string from startedAt ISO timestamp. */
function elapsedSince(startedAt: string): string {
  const start = new Date(startedAt).getTime();
  if (Number.isNaN(start)) return '';
  const seconds = Math.floor((Date.now() - start) / 1000);
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
  return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
}

export function InteractiveSessionRail({
  sessions,
  selectedSessionId,
  pollErrors,
  reduceMotion = false,
  onSelectSession,
  onStopSession,
  onRemoveSession,
  onClearStoppedSessions,
}: InteractiveSessionRailProps) {
  // Build adapter instance index for lane labels (M4.8.2)
  const instanceIndex = useMemo(() => {
    const counts = new Map<string, number>();
    const index = new Map<string, number>();
    for (const s of sessions) {
      const count = (counts.get(s.agentKey) ?? 0) + 1;
      counts.set(s.agentKey, count);
      index.set(s.sessionId, count);
    }
    // Only show index when adapter has >1 session
    const result = new Map<string, number | null>();
    for (const s of sessions) {
      result.set(s.sessionId, (counts.get(s.agentKey) ?? 0) > 1 ? index.get(s.sessionId)! : null);
    }
    return result;
  }, [sessions]);

  const containerStyle: CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    gap: 'var(--space-2)',
    height: '100%',
  };

  const runningCount = sessions.filter((s) => s.status === 'running').length;
  const stoppedCount = sessions.length - runningCount;

  return (
    <div style={containerStyle} data-testid="session-rail">
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
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
          Threads
        </span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
          {stoppedCount > 0 && (
            <button
              type="button"
              onClick={onClearStoppedSessions}
              data-testid="clear-stopped-sessions"
              style={{
                border: '1px solid var(--color-border-700)',
                borderRadius: 'var(--radius-sm)',
                backgroundColor: 'var(--color-surface-800)',
                color: 'var(--color-text-muted)',
                fontSize: '10px',
                padding: '2px var(--space-1)',
                cursor: 'pointer',
                whiteSpace: 'nowrap',
              }}
            >
              Clear stopped
            </button>
          )}
          {runningCount > 0 && (
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-marine-400)' }}>
              {runningCount} active
            </span>
          )}
        </div>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 'var(--space-1)' }}>
        {sessions.length === 0 && (
          <div
            style={{
              color: 'var(--color-text-muted)',
              fontSize: 'var(--text-xs)',
              padding: 'var(--space-3) var(--space-2)',
              textAlign: 'center',
            }}
            data-testid="empty-session-state"
          >
            No threads. Launch one from the create panel.
          </div>
        )}

        {sessions.map((session) => (
          <LaneCard
            key={session.sessionId}
            session={session}
            instanceIndex={instanceIndex.get(session.sessionId) ?? null}
            selected={selectedSessionId === session.sessionId}
            pollError={pollErrors.get(session.sessionId) ?? null}
            reduceMotion={reduceMotion}
            onSelect={() => onSelectSession(session.sessionId)}
            onStop={() => onStopSession(session.sessionId)}
            onRemove={() => onRemoveSession(session.sessionId)}
          />
        ))}
      </div>
    </div>
  );
}

function LaneCard({
  session,
  instanceIndex,
  selected,
  pollError,
  reduceMotion,
  onSelect,
  onStop,
  onRemove,
}: {
  session: InteractiveSessionSummary;
  instanceIndex: number | null;
  selected: boolean;
  pollError: string | null;
  reduceMotion: boolean;
  onSelect: () => void;
  onStop: () => void;
  onRemove: () => void;
}) {
  const [hovered, setHovered] = useState(false);
  const lifecycle = toLifecycle(session.status);
  const isRunning = lifecycle === 'running';
  const elapsed = elapsedSince(session.startedAt);

  const cardStyle: CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    gap: 'var(--space-1)',
    padding: 'var(--space-2)',
    borderRadius: 'var(--radius-md)',
    cursor: 'pointer',
    transition: reduceMotion ? 'none' : 'all var(--transition-fast)',
    border: selected
      ? '1px solid var(--color-marine-500)'
      : '1px solid transparent',
    backgroundColor: selected
      ? 'color-mix(in srgb, var(--color-marine-500) 10%, var(--color-surface-800))'
      : hovered
        ? 'var(--color-surface-750)'
        : 'transparent',
  };

  return (
    <div
      style={cardStyle}
      onClick={onSelect}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      data-testid={`session-item-${session.sessionId}`}
    >
      {/* Row 1: adapter key + instance index + lifecycle badge */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 'var(--space-1)' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-1)', minWidth: 0 }}>
          {isRunning && <PulsingDot reduceMotion={reduceMotion} />}
          <span
            style={{
              fontSize: 'var(--text-sm)',
              fontWeight: selected
                ? ('var(--weight-semibold)' as unknown as number)
                : ('var(--weight-normal)' as unknown as number),
              color: selected ? 'var(--color-text-primary)' : 'var(--color-text-secondary)',
              fontFamily: 'var(--font-mono)',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
            data-testid={`lane-label-${session.sessionId}`}
          >
            {session.agentKey}
            {instanceIndex !== null && (
              <span style={{ color: 'var(--color-text-muted)' }}> #{instanceIndex}</span>
            )}
          </span>
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
              title="Stop thread"
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
          {!isRunning && (
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onRemove();
              }}
              title="Remove thread"
              data-testid={`remove-session-${session.sessionId}`}
              style={{
                background: 'none',
                border: 'none',
                color: 'var(--color-text-muted)',
                cursor: 'pointer',
                fontSize: 'var(--text-sm)',
                padding: '2px',
                lineHeight: 1,
              }}
            >
              ✕
            </button>
          )}
        </div>
      </div>

      {/* Row 2: session ID snippet + event count + elapsed */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          fontSize: 'var(--text-xs)',
          color: 'var(--color-text-muted)',
        }}
      >
        <span style={{ fontFamily: 'var(--font-mono)' }}>
          {session.sessionId.slice(0, 8)}
        </span>
        <span>
          {session.eventCount} evts{elapsed ? ` · ${elapsed}` : ''}
        </span>
      </div>

      {/* Row 3: effective thread path + worktree marker */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: 'var(--space-1)',
        }}
      >
        <span
          style={{
            fontSize: '10px',
            color: 'var(--color-text-muted)',
            fontFamily: 'var(--font-mono)',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            minWidth: 0,
          }}
          title={session.effectiveCwd}
          data-testid={`lane-path-${session.sessionId}`}
        >
          {session.effectiveCwd}
        </span>
        {session.worktreePath && (
          <span
            style={{
              fontSize: '10px',
              color: 'var(--color-warning-400)',
              border: '1px solid color-mix(in srgb, var(--color-warning-500) 40%, transparent)',
              borderRadius: 'var(--radius-sm)',
              padding: '0 var(--space-1)',
              flexShrink: 0,
            }}
            title={session.worktreePath}
            data-testid={`lane-worktree-${session.sessionId}`}
          >
            wt
          </span>
        )}
      </div>

      {/* Row 4: poll error indicator (M4.8.5) */}
      {pollError && (
        <div
          style={{
            fontSize: '10px',
            color: 'var(--color-warning-400)',
            padding: '2px var(--space-1)',
            borderRadius: 'var(--radius-sm)',
            backgroundColor: 'color-mix(in srgb, var(--color-warning-500) 8%, transparent)',
          }}
          data-testid={`lane-error-${session.sessionId}`}
        >
          Poll error
        </div>
      )}
    </div>
  );
}

function PulsingDot({ reduceMotion }: { reduceMotion: boolean }) {
  const dotStyle: CSSProperties = {
    width: 8,
    height: 8,
    borderRadius: '50%',
    backgroundColor: 'var(--color-marine-400)',
    flexShrink: 0,
    animation: reduceMotion ? 'none' : 'pulse-dot 1.5s ease-in-out infinite',
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
