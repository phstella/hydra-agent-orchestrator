import { useEffect, useRef, useMemo } from 'react';
import type { CSSProperties } from 'react';
import type { InteractiveStreamEvent } from '../types';
import { Badge } from './design-system';

interface InteractiveTerminalPanelProps {
  sessionId: string | null;
  agentKey: string | null;
  status: string | null;
  events: InteractiveStreamEvent[];
}

const VISIBLE_TAIL = 500;

export function InteractiveTerminalPanel({
  sessionId,
  agentKey,
  status,
  events,
}: InteractiveTerminalPanelProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const userScrolledUp = useRef(false);

  const visibleEvents = useMemo(() => {
    if (events.length <= VISIBLE_TAIL) return events;
    return events.slice(events.length - VISIBLE_TAIL);
  }, [events]);

  useEffect(() => {
    const el = scrollRef.current;
    if (!el || userScrolledUp.current) return;
    el.scrollTop = el.scrollHeight;
  }, [visibleEvents]);

  const handleScroll = () => {
    const el = scrollRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
    userScrolledUp.current = !atBottom;
  };

  const containerStyle: CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    flex: 1,
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

  const scrollAreaStyle: CSSProperties = {
    flex: 1,
    overflowY: 'auto',
    padding: 'var(--space-3)',
    backgroundColor: 'var(--color-bg-950)',
    fontFamily: 'var(--font-mono)',
    fontSize: 'var(--text-xs)',
    lineHeight: 'var(--leading-relaxed)',
  };

  const statusVariant = status
    ? ({ running: 'info', completed: 'success', failed: 'danger', stopped: 'warning' } as Record<string, 'info' | 'success' | 'danger' | 'warning'>)[status] ?? 'neutral'
    : undefined;

  if (!sessionId) {
    return (
      <div style={containerStyle}>
        <div style={headerStyle}>
          <span style={titleStyle}>Terminal Output</span>
        </div>
        <div
          style={{
            ...scrollAreaStyle,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
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
        <span style={titleStyle}>
          {agentKey ? `Terminal: ${agentKey}` : 'Terminal Output'}
        </span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
          {events.length > VISIBLE_TAIL && (
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>
              showing last {VISIBLE_TAIL} of {events.length}
            </span>
          )}
          {status && statusVariant && (
            <Badge variant={statusVariant as 'info' | 'success' | 'danger' | 'warning'} dot>{status}</Badge>
          )}
        </div>
      </div>

      <div
        ref={scrollRef}
        style={scrollAreaStyle}
        onScroll={handleScroll}
        data-testid="terminal-output"
      >
        {visibleEvents.length === 0 ? (
          <div style={{ color: 'var(--color-text-muted)', padding: 'var(--space-4)' }}>
            Waiting for output...
          </div>
        ) : (
          visibleEvents.map((evt, idx) => (
            <TerminalLine key={`${evt.timestamp}-${idx}`} event={evt} />
          ))
        )}
      </div>
    </div>
  );
}

function TerminalLine({ event }: { event: InteractiveStreamEvent }) {
  const text = extractText(event);
  if (!text) return null;

  const isInput = event.eventType === 'user_input';
  const isSystem = event.eventType === 'session_started' ||
    event.eventType === 'session_completed' ||
    event.eventType === 'session_failed' ||
    event.eventType === 'session_stopped';

  const lineStyle: CSSProperties = {
    whiteSpace: 'pre-wrap',
    wordBreak: 'break-all',
    padding: '1px 0',
    color: isInput
      ? 'var(--color-marine-400)'
      : isSystem
        ? 'var(--color-text-muted)'
        : 'var(--color-text-secondary)',
  };

  return (
    <div style={lineStyle}>
      {isInput && <span style={{ color: 'var(--color-marine-300)', marginRight: 'var(--space-1)' }}>▸ </span>}
      {text}
    </div>
  );
}

function extractText(event: InteractiveStreamEvent): string {
  if (!event.data || typeof event.data !== 'object') {
    if (typeof event.data === 'string') return event.data;
    return '';
  }
  const data = event.data as Record<string, unknown>;
  if (typeof data.text === 'string') return data.text;
  if (typeof data.line === 'string') return data.line;
  if (typeof data.input === 'string') return data.input;
  if (typeof data.message === 'string') return data.message;
  if (event.eventType === 'session_started') return '--- Session started ---';
  if (event.eventType === 'session_completed') return '--- Session completed ---';
  if (event.eventType === 'session_failed') return '--- Session failed ---';
  if (event.eventType === 'session_stopped') return '--- Session stopped ---';
  const keys = Object.keys(data);
  if (keys.length === 0) return '';
  return JSON.stringify(data);
}
