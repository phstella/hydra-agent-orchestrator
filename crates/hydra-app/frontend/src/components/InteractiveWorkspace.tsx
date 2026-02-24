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
  listAdapters,
} from '../ipc';
import type {
  InteractiveSessionSummary,
  InteractiveStreamEvent,
  AdapterInfo,
} from '../types';

const MAX_CLIENT_EVENTS_PER_SESSION = 5_000;
const POLL_INTERVAL_MS = 250;
const POLL_RETRY_MS = 1_000;

function appendBoundedEvents(
  existing: InteractiveStreamEvent[],
  incoming: InteractiveStreamEvent[],
): InteractiveStreamEvent[] {
  const merged = [...existing, ...incoming];
  if (merged.length <= MAX_CLIENT_EVENTS_PER_SESSION) return merged;
  return merged.slice(merged.length - MAX_CLIENT_EVENTS_PER_SESSION);
}

function isAdapterSelectable(adapter: AdapterInfo): boolean {
  return (adapter.tier === 'tier1' && adapter.status === 'ready')
    || (adapter.tier === 'experimental' && (adapter.status === 'experimental_ready' || adapter.status === 'ready'));
}

function parseGatingErrorCode(errorStr: string): string | null {
  const match = errorStr.match(/^\[([\w_]+)\]/);
  return match ? match[1] : null;
}

interface InteractiveWorkspaceProps {
  workspaceCwd: string | null;
}

export function InteractiveWorkspace({ workspaceCwd }: InteractiveWorkspaceProps) {
  const [sessions, setSessions] = useState<InteractiveSessionSummary[]>([]);
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [sessionEvents, setSessionEvents] = useState<Map<string, InteractiveStreamEvent[]>>(new Map());
  const [pollErrors, setPollErrors] = useState<Map<string, string>>(new Map());
  const [creating, setCreating] = useState(false);
  const [showNewForm, setShowNewForm] = useState(false);
  const [agentKey, setAgentKey] = useState('');
  const [allAdapters, setAllAdapters] = useState<AdapterInfo[]>([]);
  const [availableAgents, setAvailableAgents] = useState<string[]>([]);
  const [agentLoadError, setAgentLoadError] = useState<string | null>(null);
  const [taskPrompt, setTaskPrompt] = useState('');
  const [createError, setCreateError] = useState<string | null>(null);
  const [createErrorCode, setCreateErrorCode] = useState<string | null>(null);
  const [allowExperimental, setAllowExperimental] = useState(false);
  const [experimentalAcknowledged, setExperimentalAcknowledged] = useState(false);
  const [unsafeMode, setUnsafeMode] = useState(false);

  const pollTimers = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());
  const pollCursors = useRef<Map<string, number>>(new Map());
  const pollingSessions = useRef<Set<string>>(new Set());

  useEffect(() => {
    let cancelled = false;

    async function loadInteractiveContext() {
      const [sessionResult, adapterResult] = await Promise.allSettled([
        listInteractiveSessions(),
        listAdapters(),
      ]);

      if (cancelled) return;

      if (sessionResult.status === 'fulfilled') {
        setSessions(sessionResult.value);
        if (sessionResult.value.length > 0) {
          setSelectedSessionId((prev) => prev ?? sessionResult.value[0]?.sessionId ?? null);
        }
      }

      if (adapterResult.status === 'fulfilled') {
        setAllAdapters(adapterResult.value);
        const keys = adapterResult.value
          .filter(isAdapterSelectable)
          .map((adapter) => adapter.key);
        setAvailableAgents(keys);
        setAgentLoadError(null);
        if (keys.length > 0) {
          setAgentKey((prev) => (prev && keys.includes(prev) ? prev : keys[0]));
        }
      } else {
        setAgentLoadError('Unable to load adapters. Using fallback defaults.');
        setAllAdapters([]);
        setAvailableAgents(['claude', 'codex']);
        setAgentKey((prev) => prev || 'claude');
      }
    }

    loadInteractiveContext().catch(() => {
      if (!cancelled) {
        setAgentLoadError('Unable to load interactive context.');
        setAllAdapters([]);
        setAvailableAgents(['claude', 'codex']);
        setAgentKey((prev) => prev || 'claude');
      }
    });

    return () => {
      cancelled = true;
    };
  }, []);

  const startPolling = useCallback((sessionId: string) => {
    if (pollingSessions.current.has(sessionId)) return;
    pollingSessions.current.add(sessionId);
    if (!pollCursors.current.has(sessionId)) {
      pollCursors.current.set(sessionId, 0);
    }

    function poll() {
      const cursor = pollCursors.current.get(sessionId) ?? 0;

      pollInteractiveEvents(sessionId, cursor)
        .then((batch) => {
          setPollErrors((prev) => {
            if (!prev.has(sessionId)) return prev;
            const next = new Map(prev);
            next.delete(sessionId);
            return next;
          });

          pollCursors.current.set(sessionId, batch.nextCursor);

          if (batch.events.length > 0) {
            setSessionEvents((prev) => {
              const next = new Map(prev);
              const existing = next.get(sessionId) ?? [];
              next.set(sessionId, appendBoundedEvents(existing, batch.events));
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
            pollingSessions.current.delete(sessionId);
            return;
          }

          const timer = setTimeout(poll, POLL_INTERVAL_MS);
          pollTimers.current.set(sessionId, timer);
        })
        .catch((err) => {
          const errorMessage = err instanceof Error ? err.message : String(err);
          setPollErrors((prev) => {
            const next = new Map(prev);
            next.set(sessionId, errorMessage || 'Stream polling failed');
            return next;
          });

          if (errorMessage.toLowerCase().includes('not found')) {
            pollTimers.current.delete(sessionId);
            pollingSessions.current.delete(sessionId);
            setSessions((prev) =>
              prev.map((s) =>
                s.sessionId === sessionId ? { ...s, status: 'failed' } : s,
              ),
            );
            return;
          }

          const timer = setTimeout(poll, POLL_RETRY_MS);
          pollTimers.current.set(sessionId, timer);
        });
    }

    poll();
  }, []);

  useEffect(() => {
    sessions.forEach((session) => {
      if (session.status === 'running') {
        startPolling(session.sessionId);
      } else {
        const timer = pollTimers.current.get(session.sessionId);
        if (timer) {
          clearTimeout(timer);
          pollTimers.current.delete(session.sessionId);
          pollingSessions.current.delete(session.sessionId);
        }
      }
    });
  }, [sessions, startPolling]);

  const handleCreateSession = useCallback(() => {
    setShowNewForm(true);
    setCreateError(null);
    setCreateErrorCode(null);
    setExperimentalAcknowledged(false);
  }, []);

  const selectedAdapterInfo = allAdapters.find((a) => a.key === agentKey) ?? null;
  const selectedIsExperimental = selectedAdapterInfo?.tier === 'experimental';
  const needsExperimentalConfirmation = selectedIsExperimental && !experimentalAcknowledged;

  const handleConfirmCreate = useCallback(async () => {
    if (!agentKey) {
      setCreateError('Select an available agent first.');
      setCreateErrorCode(null);
      return;
    }
    if (!taskPrompt.trim()) {
      setCreateError('Enter a task prompt.');
      setCreateErrorCode(null);
      return;
    }
    if (needsExperimentalConfirmation) {
      setCreateError('You must acknowledge the experimental adapter risk before starting a session.');
      setCreateErrorCode('experimental_blocked');
      return;
    }

    setCreating(true);
    setCreateError(null);
    setCreateErrorCode(null);

    try {
      const result = await startInteractiveSession({
        agentKey,
        taskPrompt: taskPrompt.trim(),
        allowExperimental: allowExperimental && experimentalAcknowledged,
        unsafeMode,
        cwd: workspaceCwd,
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
      setUnsafeMode(false);
      setAllowExperimental(false);
      setExperimentalAcknowledged(false);
      startPolling(result.sessionId);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setCreateError(errorMessage);
      setCreateErrorCode(parseGatingErrorCode(errorMessage));
    } finally {
      setCreating(false);
    }
  }, [agentKey, taskPrompt, startPolling, allowExperimental, experimentalAcknowledged, unsafeMode, needsExperimentalConfirmation, workspaceCwd]);

  const handleStopSession = useCallback(async (sessionId: string) => {
    try {
      const result = await stopInteractiveSession(sessionId);
      const timer = pollTimers.current.get(sessionId);
      if (timer) {
        clearTimeout(timer);
        pollTimers.current.delete(sessionId);
      }
      pollingSessions.current.delete(sessionId);
      setPollErrors((prev) => {
        if (!prev.has(sessionId)) return prev;
        const next = new Map(prev);
        next.delete(sessionId);
        return next;
      });
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
  const selectedPollError = selectedSessionId
    ? (pollErrors.get(selectedSessionId) ?? null)
    : null;

  useEffect(() => {
    return () => {
      pollTimers.current.forEach((timer) => clearTimeout(timer));
      pollTimers.current.clear();
      pollCursors.current.clear();
      pollingSessions.current.clear();
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
            <div style={{ display: 'flex', gap: 'var(--space-2)', flexWrap: 'wrap' }}>
              {availableAgents.map((key) => {
                const adapterInfo = allAdapters.find((a) => a.key === key);
                const isExp = adapterInfo?.tier === 'experimental';
                return (
                  <button
                    key={key}
                    type="button"
                    onClick={() => {
                      setAgentKey(key);
                      if (isExp) {
                        setAllowExperimental(true);
                      } else {
                        setAllowExperimental(false);
                        setExperimentalAcknowledged(false);
                      }
                      setCreateError(null);
                      setCreateErrorCode(null);
                    }}
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
                    <Badge variant={isExp ? 'warning' : 'success'}>
                      {isExp ? 'Experimental' : 'Tier-1'}
                    </Badge>
                  </button>
                );
              })}
              {availableAgents.length === 0 && (
                <span style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-muted)' }}>
                  No adapters available
                </span>
              )}
            </div>
            {agentLoadError && (
              <div style={{ marginTop: 'var(--space-2)', color: 'var(--color-warning-400)', fontSize: 'var(--text-xs)' }}>
                {agentLoadError}
              </div>
            )}
          </div>

          {selectedIsExperimental && (
            <div
              style={{
                marginBottom: 'var(--space-3)',
                padding: 'var(--space-3)',
                borderRadius: 'var(--radius-md)',
                border: '1px solid var(--color-warning-400)',
                backgroundColor: 'color-mix(in srgb, var(--color-warning-400) 8%, transparent)',
              }}
              data-testid="experimental-warning"
            >
              <div style={{ fontSize: 'var(--text-sm)', fontWeight: 'var(--weight-semibold)' as unknown as number, color: 'var(--color-warning-400)', marginBottom: 'var(--space-2)' }}>
                Experimental Adapter Warning
              </div>
              <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-secondary)', marginBottom: 'var(--space-2)' }}>
                This adapter is experimental and may produce unstable results, consume resources unpredictably, or fail without clear error messages. Use at your own risk.
              </div>
              <label style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)', fontSize: 'var(--text-sm)', cursor: 'pointer' }}>
                <input
                  type="checkbox"
                  checked={experimentalAcknowledged}
                  onChange={(e) => setExperimentalAcknowledged(e.target.checked)}
                  data-testid="experimental-acknowledge-checkbox"
                />
                <span style={{ color: 'var(--color-text-primary)' }}>I understand the risks and want to proceed</span>
              </label>
            </div>
          )}

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
            <div
              style={{
                marginBottom: 'var(--space-2)',
                fontSize: 'var(--text-xs)',
                color: 'var(--color-text-muted)',
              }}
              data-testid="interactive-workspace-path"
            >
              Workspace: {workspaceCwd ?? '(current repository)'}
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

          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-3)', marginBottom: 'var(--space-3)' }}>
            <Button
              variant="primary"
              onClick={handleConfirmCreate}
              loading={creating}
              disabled={availableAgents.length === 0 || (selectedIsExperimental && !experimentalAcknowledged)}
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
                color: createErrorCode === 'dirty_worktree'
                  ? 'var(--color-warning-400)'
                  : 'var(--color-danger-400)',
                fontSize: 'var(--text-sm)',
                padding: 'var(--space-2) var(--space-3)',
                borderRadius: 'var(--radius-md)',
                backgroundColor: createErrorCode
                  ? 'color-mix(in srgb, var(--color-danger-400) 8%, transparent)'
                  : 'transparent',
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
              const session = sessions.find((entry) => entry.sessionId === id);
              if (!session || session.status === 'running') {
                startPolling(id);
              }
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
            transportError={selectedPollError}
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
