import { useEffect, useRef, useMemo } from 'react';
import type { CSSProperties } from 'react';
import type { AgentStreamEvent } from '../types';
import type { AgentLifecycle } from '../hooks/useAgentStatuses';
import { Badge } from './design-system';

interface LiveOutputPanelProps {
  agentKey: string | null;
  lifecycle: AgentLifecycle | null;
  events: AgentStreamEvent[];
  eventsByAgent: (agentKey: string) => AgentStreamEvent[];
}

const VISIBLE_TAIL = 200;

export function LiveOutputPanel({
  agentKey,
  lifecycle,
  events,
  eventsByAgent,
}: LiveOutputPanelProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const userScrolledUp = useRef(false);

  const agentEvents = useMemo(() => {
    if (!agentKey) return events;
    return eventsByAgent(agentKey);
  }, [agentKey, events, eventsByAgent]);

  const visibleEvents = useMemo(() => {
    if (agentEvents.length <= VISIBLE_TAIL) return agentEvents;
    return agentEvents.slice(agentEvents.length - VISIBLE_TAIL);
  }, [agentEvents]);

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
    color: agentKey ? 'var(--color-text-primary)' : 'var(--color-text-muted)',
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

  const lifecycleVariant = lifecycle
    ? ({ running: 'info', completed: 'success', failed: 'danger', timed_out: 'warning' } as const)[lifecycle]
    : undefined;

  return (
    <div style={containerStyle}>
      <div style={headerStyle}>
        <span style={titleStyle}>
          {agentKey ? `Output: ${agentKey}` : 'Live Output'}
        </span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
          {agentEvents.length > VISIBLE_TAIL && (
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>
              showing last {VISIBLE_TAIL} of {agentEvents.length}
            </span>
          )}
          {lifecycle && lifecycleVariant && (
            <Badge variant={lifecycleVariant} dot>{lifecycle.replace('_', ' ')}</Badge>
          )}
        </div>
      </div>

      <div ref={scrollRef} style={scrollAreaStyle} onScroll={handleScroll}>
        {visibleEvents.length === 0 ? (
          <div style={{ color: 'var(--color-text-muted)', padding: 'var(--space-4)' }}>
            {agentKey ? `Waiting for output from ${agentKey}...` : 'No events yet.'}
          </div>
        ) : (
          visibleEvents.map((evt, idx) => (
            <OutputLine key={`${evt.timestamp}-${idx}`} event={evt} showAgent={!agentKey} />
          ))
        )}
      </div>
    </div>
  );
}

function OutputLine({ event, showAgent }: { event: AgentStreamEvent; showAgent: boolean }) {
  const lineStyle: CSSProperties = {
    display: 'flex',
    gap: 'var(--space-2)',
    padding: '1px 0',
    whiteSpace: 'pre-wrap',
    wordBreak: 'break-all',
  };

  const timestampStyle: CSSProperties = {
    color: 'var(--color-text-muted)',
    flexShrink: 0,
    minWidth: 72,
  };

  const agentStyle: CSSProperties = {
    color: 'var(--color-marine-400)',
    flexShrink: 0,
    minWidth: 80,
  };

  const typeStyle: CSSProperties = {
    color: eventTypeColor(event.eventType),
    flexShrink: 0,
    minWidth: 120,
  };

  const dataContent = extractDisplayText(event);

  const maybeEpochSeconds = Number(event.timestamp);
  const parsedTime = Number.isFinite(maybeEpochSeconds)
    ? new Date(maybeEpochSeconds * 1000)
    : new Date(event.timestamp);
  const timeStr = Number.isNaN(parsedTime.getTime())
    ? event.timestamp.slice(0, 12)
    : parsedTime.toLocaleTimeString();

  return (
    <div style={lineStyle}>
      <span style={timestampStyle}>{timeStr}</span>
      {showAgent && <span style={agentStyle}>{event.agentKey}</span>}
      <span style={typeStyle}>{event.eventType}</span>
      {dataContent && (
        <span style={{ color: 'var(--color-text-secondary)', flex: 1 }}>{dataContent}</span>
      )}
    </div>
  );
}

function eventTypeColor(eventType: string): string {
  if (eventType.includes('started')) return 'var(--color-green-400)';
  if (eventType.includes('completed')) return 'var(--color-green-500)';
  if (eventType.includes('failed') || eventType.includes('error')) return 'var(--color-danger-400)';
  if (eventType.includes('timeout') || eventType.includes('timed_out')) return 'var(--color-warning-400)';
  if (eventType.includes('stdout') || eventType.includes('output')) return 'var(--color-text-secondary)';
  return 'var(--color-marine-400)';
}

function extractDisplayText(event: AgentStreamEvent): string {
  if (!event.data || typeof event.data !== 'object') return '';
  const data = event.data as Record<string, unknown>;
  if (typeof data.line === 'string') return data.line;
  if (typeof data.message === 'string') return data.message;
  if (typeof data.text === 'string') return data.text;
  const keys = Object.keys(data);
  if (keys.length === 0) return '';
  return JSON.stringify(data);
}
