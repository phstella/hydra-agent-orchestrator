import { useEffect, useRef, useMemo, useState } from 'react';
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
type OutputViewMode = 'human' | 'events';

export function LiveOutputPanel({
  agentKey,
  lifecycle,
  events,
  eventsByAgent,
}: LiveOutputPanelProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const userScrolledUp = useRef(false);
  const [viewMode, setViewMode] = useState<OutputViewMode>('human');

  const agentEvents = useMemo(() => {
    if (!agentKey) return events;
    return eventsByAgent(agentKey);
  }, [agentKey, events, eventsByAgent]);

  const visibleEvents = useMemo(() => {
    if (agentEvents.length <= VISIBLE_TAIL) return agentEvents;
    return agentEvents.slice(agentEvents.length - VISIBLE_TAIL);
  }, [agentEvents]);

  const displayEvents = useMemo(() => {
    if (viewMode === 'events') return visibleEvents;
    return visibleEvents.filter((evt) => extractHumanReadableText(evt).length > 0);
  }, [viewMode, visibleEvents]);

  useEffect(() => {
    const el = scrollRef.current;
    if (!el || userScrolledUp.current) return;
    el.scrollTop = el.scrollHeight;
  }, [displayEvents]);

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
    <div style={containerStyle} data-testid="live-output-panel">
      <div style={headerStyle}>
        <span style={titleStyle}>
          {agentKey ? `Output: ${agentKey}` : 'Live Output'}
        </span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
          <div style={{ display: 'inline-flex', gap: 'var(--space-1)' }}>
            <OutputModeButton
              active={viewMode === 'human'}
              onClick={() => setViewMode('human')}
              testId="output-view-human"
            >
              Human
            </OutputModeButton>
            <OutputModeButton
              active={viewMode === 'events'}
              onClick={() => setViewMode('events')}
              testId="output-view-events"
            >
              Events
            </OutputModeButton>
          </div>
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
        {displayEvents.length === 0 ? (
          <div style={{ color: 'var(--color-text-muted)', padding: 'var(--space-4)' }}>
            {agentKey ? `Waiting for output from ${agentKey}...` : 'No events yet.'}
          </div>
        ) : (
          displayEvents.map((evt, idx) => (
            <OutputLine
              key={`${evt.timestamp}-${idx}`}
              event={evt}
              showAgent={!agentKey}
              viewMode={viewMode}
            />
          ))
        )}
      </div>
    </div>
  );
}

function OutputModeButton({
  active,
  onClick,
  testId,
  children,
}: {
  active: boolean;
  onClick: () => void;
  testId: string;
  children: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      data-testid={testId}
      style={{
        border: active ? '1px solid var(--color-marine-500)' : '1px solid var(--color-border-700)',
        backgroundColor: active
          ? 'color-mix(in srgb, var(--color-marine-500) 12%, transparent)'
          : 'var(--color-surface-800)',
        color: active ? 'var(--color-marine-400)' : 'var(--color-text-secondary)',
        fontSize: 'var(--text-xs)',
        borderRadius: 'var(--radius-sm)',
        padding: '2px var(--space-2)',
        cursor: 'pointer',
        fontFamily: 'var(--font-family)',
      }}
    >
      {children}
    </button>
  );
}

function OutputLine({
  event,
  showAgent,
  viewMode,
}: {
  event: AgentStreamEvent;
  showAgent: boolean;
  viewMode: OutputViewMode;
}) {
  const lineStyle: CSSProperties = {
    display: 'flex',
    gap: 'var(--space-2)',
    padding: '1px 0',
    whiteSpace: 'pre-wrap',
    wordBreak: 'break-word',
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

  const dataContent = viewMode === 'human' ? extractHumanReadableText(event) : extractDisplayText(event);

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
      {viewMode === 'events' && <span style={typeStyle}>{event.eventType}</span>}
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

function extractHumanReadableText(event: AgentStreamEvent): string {
  switch (event.eventType) {
    case 'agent_started':
      return 'Agent started.';
    case 'agent_completed': {
      if (typeof event.data === 'object' && event.data !== null) {
        const data = event.data as Record<string, unknown>;
        const tokens = typeof data.total_tokens === 'number' ? ` (${data.total_tokens} tokens)` : '';
        return `Agent completed${tokens}.`;
      }
      return 'Agent completed.';
    }
    case 'agent_failed': {
      if (typeof event.data === 'object' && event.data !== null) {
        const data = event.data as Record<string, unknown>;
        const err = typeof data.error === 'string' ? summarizeText(data.error, 200) : null;
        return err ? `Agent failed: ${err}` : 'Agent failed.';
      }
      return 'Agent failed.';
    }
    case 'agent_timed_out':
    case 'agent_timeout':
      return 'Agent timed out.';
    default:
      break;
  }

  if (typeof event.data === 'string') {
    return summarizeText(event.data);
  }

  if (typeof event.data !== 'object' || event.data === null) {
    return '';
  }

  const data = event.data as Record<string, unknown>;
  if (typeof data.content === 'string') {
    return summarizeText(data.content);
  }
  if (typeof data.message === 'string') {
    return summarizeText(data.message);
  }
  if (typeof data.text === 'string') {
    return summarizeText(data.text);
  }

  if (typeof data.line === 'string') {
    const parsed = parseStructuredOutputLine(data.line);
    if (parsed) return parsed;
    const normalized = summarizeText(data.line);
    if (looksLikeJson(normalized)) return '';
    return normalized;
  }

  if (typeof data.error === 'string') {
    return summarizeText(data.error);
  }

  return '';
}

function parseStructuredOutputLine(line: string): string | null {
  const normalized = normalizeTerminalText(line);
  if (!looksLikeJson(normalized)) return summarizeText(normalized);

  try {
    const parsed = JSON.parse(normalized) as unknown;
    return humanizeStructuredPayload(parsed);
  } catch {
    return summarizeText(normalized);
  }
}

function humanizeStructuredPayload(payload: unknown): string | null {
  const record = asRecord(payload);
  if (!record) return null;

  if (typeof record.result === 'string') {
    return summarizeText(record.result);
  }
  if (typeof record.text === 'string') {
    return summarizeText(record.text);
  }
  if (typeof record.message === 'string') {
    return summarizeText(record.message);
  }
  if (typeof record.content === 'string') {
    return summarizeText(record.content);
  }

  const payloadType = typeof record.type === 'string' ? record.type : null;
  if (payloadType === 'item.completed') {
    const item = asRecord(record.item);
    if (item && typeof item.text === 'string') {
      return summarizeText(item.text);
    }
  }

  if (payloadType === 'assistant') {
    const message = asRecord(record.message);
    const rendered = renderAssistantContent(message);
    if (rendered) return rendered;
  }

  if (payloadType === 'user') {
    const message = asRecord(record.message);
    const rendered = renderUserToolResults(record, message);
    if (rendered) return rendered;
  }

  return null;
}

function renderAssistantContent(message: Record<string, unknown> | null): string | null {
  if (!message) return null;
  const content = message.content;
  if (!Array.isArray(content)) return null;

  const lines: string[] = [];
  for (const part of content) {
    const entry = asRecord(part);
    if (!entry) continue;
    if (typeof entry.text === 'string') {
      lines.push(summarizeText(entry.text));
      continue;
    }
    if (entry.type === 'tool_use') {
      const toolName = typeof entry.name === 'string' ? entry.name : 'tool';
      const input = asRecord(entry.input);
      const filePath = input && typeof input.file_path === 'string' ? input.file_path : null;
      if (filePath) {
        const fileName = filePath.split('/').pop() ?? filePath;
        lines.push(`Tool ${toolName}: ${fileName}`);
      } else {
        lines.push(`Tool ${toolName} invoked`);
      }
    }
  }

  if (lines.length === 0) return null;
  return summarizeText(lines.join('\n'), 400);
}

function renderUserToolResults(
  root: Record<string, unknown>,
  message: Record<string, unknown> | null,
): string | null {
  if (!message) return null;
  const content = message.content;
  if (!Array.isArray(content)) return null;

  for (const part of content) {
    const entry = asRecord(part);
    if (!entry) continue;
    if (entry.type === 'tool_result') {
      const text = typeof entry.content === 'string' ? entry.content : null;
      if (text) {
        const firstLine = normalizeTerminalText(text).split('\n').find((line) => line.trim().length > 0);
        if (firstLine) return summarizeText(firstLine, 220);
      }
    }
  }

  const toolUseResult = asRecord(root.tool_use_result);
  if (toolUseResult && typeof toolUseResult.filePath === 'string') {
    const name = toolUseResult.filePath.split('/').pop() ?? toolUseResult.filePath;
    return `Updated ${name}`;
  }

  return null;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return null;
  }
  return value as Record<string, unknown>;
}

function looksLikeJson(value: string): boolean {
  const trimmed = value.trim();
  if (trimmed.length < 2) return false;
  return (
    (trimmed.startsWith('{') && trimmed.endsWith('}')) ||
    (trimmed.startsWith('[') && trimmed.endsWith(']'))
  );
}

function summarizeText(raw: string, maxChars = 320): string {
  const normalized = normalizeTerminalText(raw).trim();
  if (!normalized) return '';

  const lines = normalized.split('\n').filter((line) => line.trim().length > 0);
  const joined = lines.join('\n');
  if (joined.length <= maxChars && lines.length <= 6) {
    return joined;
  }

  if (lines.length > 6) {
    const shown = lines.slice(0, 6).join('\n');
    const extra = lines.length - 6;
    return `${shown}\n... (${extra} more lines)`;
  }

  return `${joined.slice(0, maxChars)}...`;
}

function normalizeTerminalText(raw: string): string {
  if (!raw) return '';
  const withoutOsc = raw.replace(/\u001b\][^\u0007]*(?:\u0007|\u001b\\)/g, '');
  const withoutCsi = withoutOsc.replace(/\u001b\[[0-9;?]*[ -/]*[@-~]/g, '');
  const normalizedCr = withoutCsi
    .replace(/\r\n/g, '\n')
    .split('\n')
    .map((line) => {
      const lastCr = line.lastIndexOf('\r');
      return lastCr >= 0 ? line.slice(lastCr + 1) : line;
    })
    .join('\n');
  return normalizedCr.replace(/[\x00-\x08\x0B-\x1F\x7F]/g, '');
}
