import { useState, useCallback, useEffect, useRef } from 'react';
import type { CSSProperties } from 'react';
import { InteractiveSessionRail } from './InteractiveSessionRail';
import { InteractiveTerminalPanel } from './InteractiveTerminalPanel';
import { InputComposer } from './InputComposer';
import { Card, Button, Badge } from './design-system';
import {
  startInteractiveSession,
  pollInteractiveEvents,
  writeInteractiveInput,
  stopInteractiveSession,
  listInteractiveSessions,
} from '../ipc';
import type {
  InteractiveSessionSummary,
  InteractiveStreamEvent,
} from '../types';

export function InteractiveWorkspace() {
  const [sessions, setSessions] = useState<InteractiveSessionSummary[]>([]);
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [sessionEvents, setSessionEvents] = useState<Map<string, InteractiveStreamEvent[]>>(new Map());
  const [creating, setCreating] = useState(false);
  const [showNewForm, setShowNewForm] = useState(false);
  const [agentKey, setAgentKey] = useState('claude');
  const [taskPrompt, setTaskPrompt] = useState('');
  const [createError, setCreateError] = useState<string | null>(null);

  const pollTimers = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());
  const pollCursors = useRef<Map<string, number>>(new Map());

  useEffect(() => {
    listInteractiveSessions()
      .then(setSessions)
      .catch(() => {});
  }, []);

  const startPolling = useCallback((sessionId: string) => {
    if (pollTimers.current.has(sessionId)) return;
    pollCursors.current.set(sessionId, 0);

    function poll() {
      const cursor = pollCursors.current.get(sessionId) ?? 0;

      pollInteractiveEvents(sessionId, cursor)
        .then((batch) => {
          pollCursors.current.set(sessionId, batch.nextCursor);

          if (batch.events.length > 0) {
            setSessionEvents((prev) => {
              const next = new Map(prev);
              const existing = next.get(sessionId) ?? [];
              next.set(sessionId, [...existing, ...batch.events]);
              return next;
            });
          }

          setSessions((prev) =>
            prev.map((s) =>
              s.sessionId === sessionId
                ? { ...s, status: batch.status, eventCount: (s.eventCount ?? 0) + batch.events.length }
                : s,
            ),
          );

          if (batch.done) {
            pollTimers.current.delete(sessionId);
            return;
          }

          const timer = setTimeout(poll, 250);
          pollTimers.current.set(sessionId, timer);
        })
        .catch(() => {
          pollTimers.current.delete(sessionId);
        });
    }

    poll();
  }, []);

  const handleCreateSession = useCallback(() => {
    setShowNewForm(true);
    setCreateError(null);
  }, []);

  const handleConfirmCreate = useCallback(async () => {
    if (!taskPrompt.trim()) {
      setCreateError('Enter a task prompt.');
      return;
    }

    setCreating(true);
    setCreateError(null);

    try {
      const result = await startInteractiveSession({
        agentKey,
        taskPrompt: taskPrompt.trim(),
        allowExperimental: false,
        unsafeMode: false,
        cwd: null,
        cols: 120,
        rows: 30,
      });

      const newSession: InteractiveSessionSummary = {
        sessionId: result.sessionId,
        agentKey: result.agentKey,
        status: result.status,
        startedAt: result.startedAt,
        eventCount: 0,
      };

      setSessions((prev) => [newSession, ...prev]);
      setSelectedSessionId(result.sessionId);
      setShowNewForm(false);
      setTaskPrompt('');
      startPolling(result.sessionId);
    } catch (err) {
      setCreateError(err instanceof Error ? err.message : String(err));
    } finally {
      setCreating(false);
    }
  }, [agentKey, taskPrompt, startPolling]);

  const handleStopSession = useCallback(async (sessionId: string) => {
    try {
      const result = await stopInteractiveSession(sessionId);
      setSessions((prev) =>
        prev.map((s) =>
          s.sessionId === sessionId ? { ...s, status: result.status } : s,
        ),
      );
    } catch {
      // best-effort
    }
  }, []);

  const handleSendInput = useCallback(
    async (input: string) => {
      if (!selectedSessionId) {
        return { success: false, error: 'No session selected' };
      }
      const result = await writeInteractiveInput(selectedSessionId, input);
      return { success: result.success, error: result.error };
    },
    [selectedSessionId],
  );

  const selectedSession = sessions.find((s) => s.sessionId === selectedSessionId) ?? null;
  const selectedEvents = selectedSessionId
    ? (sessionEvents.get(selectedSessionId) ?? [])
    : [];

  useEffect(() => {
    return () => {
      pollTimers.current.forEach((timer) => clearTimeout(timer));
      pollTimers.current.clear();
    };
  }, []);

  const containerStyle: CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    maxWidth: 1200,
    margin: '0 auto',
    padding: 'var(--space-6)',
    gap: 'var(--space-4)',
    minHeight: 0,
    flex: 1,
  };

  return (
    <div style={containerStyle}>
      {showNewForm && (
        <Card padding="lg" data-testid="new-session-form">
          <h3
            style={{
              fontSize: 'var(--text-lg)',
              fontWeight: 'var(--weight-bold)' as unknown as number,
              marginBottom: 'var(--space-3)',
            }}
          >
            New Interactive Session
          </h3>

          <div style={{ marginBottom: 'var(--space-3)' }}>
            <div
              style={{
                marginBottom: 'var(--space-2)',
                fontSize: 'var(--text-sm)',
                color: 'var(--color-text-secondary)',
              }}
            >
              Agent
            </div>
            <div style={{ display: 'flex', gap: 'var(--space-2)' }}>
              {['claude', 'codex'].map((key) => (
                <button
                  key={key}
                  type="button"
                  onClick={() => setAgentKey(key)}
                  data-testid={`agent-select-${key}`}
                  style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: 'var(--space-2)',
                    borderRadius: 'var(--radius-md)',
                    border: agentKey === key
                      ? '1px solid var(--color-marine-500)'
                      : '1px solid var(--color-border-700)',
                    backgroundColor: agentKey === key
                      ? 'color-mix(in srgb, var(--color-marine-500) 12%, transparent)'
                      : 'var(--color-surface-800)',
                    color: 'var(--color-text-primary)',
                    padding: 'var(--space-2) var(--space-3)',
                    cursor: 'pointer',
                    fontFamily: 'var(--font-family)',
                    fontSize: 'var(--text-sm)',
                  }}
                >
                  {key}
                  <Badge variant="success">Tier-1</Badge>
                </button>
              ))}
            </div>
          </div>

          <div style={{ marginBottom: 'var(--space-3)' }}>
            <div
              style={{
                marginBottom: 'var(--space-2)',
                fontSize: 'var(--text-sm)',
                color: 'var(--color-text-secondary)',
              }}
            >
              Task Prompt
            </div>
            <textarea
              value={taskPrompt}
              onChange={(e) => setTaskPrompt(e.target.value)}
              placeholder="Describe what you want the agent to work on..."
              rows={3}
              data-testid="session-task-prompt"
              style={{
                width: '100%',
                borderRadius: 'var(--radius-md)',
                border: '1px solid var(--color-border-700)',
                backgroundColor: 'var(--color-bg-900)',
                color: 'var(--color-text-primary)',
                padding: 'var(--space-3)',
                resize: 'vertical',
                fontFamily: 'var(--font-family)',
                fontSize: 'var(--text-sm)',
              }}
            />
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-3)' }}>
            <Button
              variant="primary"
              onClick={handleConfirmCreate}
              loading={creating}
              data-testid="confirm-create-session"
            >
              Start Session
            </Button>
            <Button
              variant="ghost"
              onClick={() => setShowNewForm(false)}
            >
              Cancel
            </Button>
          </div>

          {createError && (
            <div
              style={{
                marginTop: 'var(--space-2)',
                color: 'var(--color-danger-400)',
                fontSize: 'var(--text-sm)',
              }}
              data-testid="create-session-error"
            >
              {createError}
            </div>
          )}
        </Card>
      )}

      <div
        style={{
          display: 'flex',
          border: '1px solid var(--color-border-700)',
          borderRadius: 'var(--radius-lg)',
          backgroundColor: 'var(--color-surface-800)',
          overflow: 'hidden',
          flex: 1,
          minHeight: 480,
        }}
      >
        <div
          style={{
            borderRight: '1px solid var(--color-border-700)',
            padding: 'var(--space-3)',
            overflowY: 'auto',
            flexShrink: 0,
          }}
        >
          <InteractiveSessionRail
            sessions={sessions}
            selectedSessionId={selectedSessionId}
            onSelectSession={(id) => {
              setSelectedSessionId(id);
              startPolling(id);
            }}
            onCreateSession={handleCreateSession}
            onStopSession={handleStopSession}
            creating={creating}
          />
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', flex: 1, minWidth: 0 }}>
          <InteractiveTerminalPanel
            sessionId={selectedSessionId}
            agentKey={selectedSession?.agentKey ?? null}
            status={selectedSession?.status ?? null}
            events={selectedEvents}
          />

          <InputComposer
            sessionId={selectedSessionId}
            sessionStatus={selectedSession?.status ?? null}
            onSendInput={handleSendInput}
            onStopSession={() => {
              if (selectedSessionId) handleStopSession(selectedSessionId);
            }}
          />
        </div>
      </div>
    </div>
  );
}
