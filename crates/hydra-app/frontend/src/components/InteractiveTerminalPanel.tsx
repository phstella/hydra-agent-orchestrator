import { useMemo, forwardRef } from 'react';
import type { CSSProperties } from 'react';
import type { InteractiveStreamEvent } from '../types';
import { Badge } from './design-system';
import { XTermRenderer } from './XTermRenderer';
import type { XTermRendererHandle } from './XTermRenderer';

interface InteractiveTerminalPanelProps {
  sessionId: string | null;
  agentKey: string | null;
  /** Disambiguated lane label, e.g. "codex · a1b2c3d4" (M4.8.2). */
  laneLabel: string | null;
  status: string | null;
  events: InteractiveStreamEvent[];
  transportError: string | null;
  sessionError: string | null;
  /** P4.9.5: Callback for terminal keyboard input routed to PTY stdin. */
  onTerminalInput?: (data: string) => void;
  /** Keep backend PTY size aligned with xterm viewport. */
  onTerminalResize?: (cols: number, rows: number) => void;
}

export const InteractiveTerminalPanel = forwardRef<XTermRendererHandle, InteractiveTerminalPanelProps>(
  function InteractiveTerminalPanel({
  sessionId,
  agentKey,
  laneLabel,
  status,
  events,
  transportError,
  sessionError,
  onTerminalInput,
  onTerminalResize,
}, ref) {
  // Extract raw text from events, preserving ANSI escape sequences.
  const chunks = useMemo(
    () => events.map(extractRawText).filter((t) => t.length > 0),
    [events],
  );

  const containerStyle: CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    flex: 1,
    minHeight: 0,
    minWidth: 0,
  };

  const headerStyle: CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: 'var(--space-2) var(--space-4)',
    borderBottom: '1px solid var(--color-border-700)',
    backgroundColor: 'var(--color-bg-900)',
    flexShrink: 0,
  };

  const titleStyle: CSSProperties = {
    fontSize: 'var(--text-sm)',
    fontWeight: 'var(--weight-semibold)' as unknown as number,
    fontFamily: 'var(--font-mono)',
    color: sessionId ? 'var(--color-text-primary)' : 'var(--color-text-muted)',
  };

  const emptyAreaStyle: CSSProperties = {
    flex: 1,
    minHeight: 0,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: 'var(--color-bg-950)',
  };

  const statusVariant = status
    ? ({
        running: 'info',
        completed: 'success',
        failed: 'danger',
        stopped: 'warning',
        paused: 'warning',
      } as Record<string, 'info' | 'success' | 'danger' | 'warning'>)[status] ?? 'neutral'
    : undefined;

  if (!sessionId) {
    return (
      <div style={containerStyle}>
        <div style={headerStyle}>
          <span style={titleStyle}>Terminal Output</span>
        </div>
        <div style={emptyAreaStyle}>
          <div
            style={{
              color: 'var(--color-text-muted)',
              textAlign: 'center',
              padding: 'var(--space-8)',
            }}
            data-testid="terminal-empty-state"
          >
            Select or create a session to see terminal output.
          </div>
        </div>
      </div>
    );
  }

  return (
    <div style={containerStyle} data-testid="terminal-panel">
      <div style={headerStyle}>
        <span style={titleStyle} data-testid="terminal-lane-label">
          {laneLabel ? `Terminal: ${laneLabel}` : agentKey ? `Terminal: ${agentKey}` : 'Terminal Output'}
        </span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
          {status && statusVariant && (
            <Badge variant={statusVariant as 'info' | 'success' | 'danger' | 'warning'} dot>{status}</Badge>
          )}
        </div>
      </div>

      {transportError && (
        <div
          style={{
            padding: 'var(--space-2) var(--space-4)',
            backgroundColor: 'color-mix(in srgb, var(--color-warning-500) 12%, transparent)',
            borderBottom: '1px solid var(--color-warning-500)',
            color: 'var(--color-warning-400)',
            fontSize: 'var(--text-xs)',
            flexShrink: 0,
          }}
          data-testid="terminal-transport-error"
        >
          Connection issue: {transportError}. Retrying...
        </div>
      )}

      {sessionError && (
        <div
          style={{
            padding: 'var(--space-2) var(--space-4)',
            backgroundColor: 'color-mix(in srgb, var(--color-danger-500) 12%, transparent)',
            borderBottom: '1px solid var(--color-danger-500)',
            color: 'var(--color-danger-300)',
            fontSize: 'var(--text-xs)',
            flexShrink: 0,
          }}
          data-testid="terminal-session-error"
        >
          Session error: {sessionError}
        </div>
      )}

      <XTermRenderer
        ref={ref}
        resetKey={sessionId}
        chunks={chunks}
        onData={status === 'running' ? onTerminalInput : undefined}
        onResize={status === 'running' ? onTerminalResize : undefined}
      />
    </div>
  );
});

/**
 * Extract raw text from an event, preserving ANSI escape sequences
 * so the xterm.js renderer can interpret them with full fidelity.
 */
function extractRawText(event: InteractiveStreamEvent): string {
  if (event.eventType === 'user_input') return '';

  if (!event.data || typeof event.data !== 'object') {
    if (typeof event.data === 'string') return event.data;
    return '';
  }
  const data = event.data as Record<string, unknown>;
  if (typeof data.text === 'string') return data.text;
  if (typeof data.line === 'string') return data.line;
  if (typeof data.message === 'string') return data.message;
  if (event.eventType === 'session_started') return '\r\n--- Session started ---\r\n';
  if (event.eventType === 'session_completed') return '\r\n--- Session completed ---\r\n';
  if (event.eventType === 'session_failed') return '\r\n--- Session failed ---\r\n';
  if (event.eventType === 'session_stopped') return '\r\n--- Session stopped ---\r\n';
  const keys = Object.keys(data);
  if (keys.length === 0) return '';
  return JSON.stringify(data);
}
