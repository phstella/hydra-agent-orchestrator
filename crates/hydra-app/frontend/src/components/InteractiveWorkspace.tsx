import { useState, useCallback, useEffect, useRef } from 'react';
import type { CSSProperties } from 'react';
import { InteractiveSessionRail } from './InteractiveSessionRail';
import { InteractiveTerminalPanel } from './InteractiveTerminalPanel';
import type { XTermRendererHandle } from './XTermRenderer';
import { Button, Badge } from './design-system';
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

/** P4.9.4: Map error codes to actionable user-facing hints. */
function errorHintForCode(code: string | null, agentKey: string): string | null {
  switch (code) {
    case 'binary_missing':
      return `Install the ${agentKey} CLI or add its path to hydra.toml.`;
    case 'launch_error':
      return 'Check that the tool binary is runnable and your auth/session is valid.';
    case 'safety_gate':
      return 'Run "hydra doctor" to diagnose adapter readiness.';
    case 'experimental_blocked':
      return 'Enable the experimental risk acknowledgment above.';
    case 'dirty_worktree':
      return 'Commit or stash your changes before deploying.';
    case 'unsafe_blocked':
      return 'This adapter does not support unsafe mode.';
    default:
      return null;
  }
}

/** Count running sessions for a given adapter key. */
function countAdapterInstances(sessions: InteractiveSessionSummary[], adapterKey: string): number {
  return sessions.filter((s) => s.agentKey === adapterKey && s.status === 'running').length;
}

interface InteractiveWorkspaceProps {
  workspaceCwd: string | null;
}

export function InteractiveWorkspace({ workspaceCwd }: InteractiveWorkspaceProps) {
  // ---------------------------------------------------------------------------
  // Session state (all keyed by session_id — M4.8.2)
  // ---------------------------------------------------------------------------
  const [sessions, setSessions] = useState<InteractiveSessionSummary[]>([]);
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [sessionEvents, setSessionEvents] = useState<Map<string, InteractiveStreamEvent[]>>(new Map());
  const [pollErrors, setPollErrors] = useState<Map<string, string>>(new Map());

  // ---------------------------------------------------------------------------
  // Create-form state
  // ---------------------------------------------------------------------------
  const [creating, setCreating] = useState(false);
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

  // ---------------------------------------------------------------------------
  // Polling refs (session_id keyed — M4.8.2)
  // ---------------------------------------------------------------------------
  const pollTimers = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());
  const pollCursors = useRef<Map<string, number>>(new Map());
  const pollingSessions = useRef<Set<string>>(new Set());

  // P4.9.5: Terminal ref for focus management
  const terminalRef = useRef<XTermRendererHandle>(null);

  // ---------------------------------------------------------------------------
  // Initial load
  // ---------------------------------------------------------------------------
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

  // ---------------------------------------------------------------------------
  // Polling — per-session isolated (M4.8.5)
  // ---------------------------------------------------------------------------
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

  // ---------------------------------------------------------------------------
  // Session creation — supports duplicate adapter instances (M4.8.3)
  // ---------------------------------------------------------------------------
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

  // ---------------------------------------------------------------------------
  // Stop — per-lane isolated (M4.8.6)
  // ---------------------------------------------------------------------------
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

  // ---------------------------------------------------------------------------
  // Input — P4.9.5: terminal-direct via onData (no side InputComposer)
  // ---------------------------------------------------------------------------
  const handleTerminalInput = useCallback(
    (data: string) => {
      if (!selectedSessionId) return;
      writeInteractiveInput(selectedSessionId, data).catch(() => {
        // best-effort; PTY write failures are surfaced via poll errors
      });
    },
    [selectedSessionId],
  );

  // ---------------------------------------------------------------------------
  // Derived state for focused lane
  // ---------------------------------------------------------------------------
  const selectedSession = sessions.find((s) => s.sessionId === selectedSessionId) ?? null;
  const selectedEvents = selectedSessionId
    ? (sessionEvents.get(selectedSessionId) ?? [])
    : [];
  const selectedPollError = selectedSessionId
    ? (pollErrors.get(selectedSessionId) ?? null)
    : null;

  // Count duplicate adapter instances for lane label disambiguation (M4.8.2)
  const runningInstanceCount = agentKey ? countAdapterInstances(sessions, agentKey) : 0;

  // ---------------------------------------------------------------------------
  // Cleanup on unmount
  // ---------------------------------------------------------------------------
  useEffect(() => {
    return () => {
      pollTimers.current.forEach((timer) => clearTimeout(timer));
      pollTimers.current.clear();
      pollCursors.current.clear();
      pollingSessions.current.clear();
    };
  }, []);

  // ---------------------------------------------------------------------------
  // P4.9.5: Focus terminal when lane selection changes
  // ---------------------------------------------------------------------------
  useEffect(() => {
    if (selectedSessionId) {
      requestAnimationFrame(() => {
        terminalRef.current?.focus();
      });
    }
  }, [selectedSessionId]);

  // ---------------------------------------------------------------------------
  // Orchestration Console Layout (M4.8.1)
  //
  // 3-column layout:
  //   Left:   Create panel (adapter, prompt, safety, launch)
  //   Center: Focused terminal + intervention controls for selected lane
  //   Right:  Running-lanes rail with per-session cards
  // ---------------------------------------------------------------------------
  const shellStyle: CSSProperties = {
    display: 'flex',
    flex: 1,
    minHeight: 0,
    overflow: 'hidden',
  };

  const leftPanelStyle: CSSProperties = {
    width: 300,
    flexShrink: 0,
    borderRight: '1px solid var(--color-border-700)',
    backgroundColor: 'var(--color-bg-900)',
    display: 'flex',
    flexDirection: 'column',
    overflowY: 'auto',
    padding: 'var(--space-4)',
    gap: 'var(--space-3)',
  };

  const centerStyle: CSSProperties = {
    flex: 1,
    minWidth: 0,
    minHeight: 0,
    display: 'flex',
    flexDirection: 'column',
    overflow: 'hidden',
  };

  const rightPanelStyle: CSSProperties = {
    width: 280,
    flexShrink: 0,
    borderLeft: '1px solid var(--color-border-700)',
    backgroundColor: 'var(--color-bg-850)',
    display: 'flex',
    flexDirection: 'column',
    overflowY: 'auto',
    padding: 'var(--space-3)',
  };

  return (
    <div style={shellStyle} data-testid="orchestration-console">
      {/* ---- Left: Create Panel ---- */}
      <div style={leftPanelStyle} data-testid="create-panel">
        <div
          style={{
            fontSize: 'var(--text-sm)',
            fontWeight: 'var(--weight-bold)' as unknown as number,
            color: 'var(--color-text-muted)',
            textTransform: 'uppercase',
            letterSpacing: '0.05em',
            marginBottom: 'var(--space-1)',
          }}
        >
          New Lane
        </div>

        {/* Agent selection */}
        <div>
          <div style={{ marginBottom: 'var(--space-2)', fontSize: 'var(--text-xs)', color: 'var(--color-text-secondary)' }}>
            Agent
          </div>
          <div style={{ display: 'flex', gap: 'var(--space-2)', flexWrap: 'wrap' }}>
            {availableAgents.map((key) => {
              const adapterInfo = allAdapters.find((a) => a.key === key);
              const isExp = adapterInfo?.tier === 'experimental';
              const instanceCount = countAdapterInstances(sessions, key);
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
                    gap: 'var(--space-1)',
                    borderRadius: 'var(--radius-md)',
                    border: agentKey === key
                      ? '1px solid var(--color-marine-500)'
                      : '1px solid var(--color-border-700)',
                    backgroundColor: agentKey === key
                      ? 'color-mix(in srgb, var(--color-marine-500) 12%, transparent)'
                      : 'var(--color-surface-800)',
                    color: 'var(--color-text-primary)',
                    padding: 'var(--space-1) var(--space-2)',
                    cursor: 'pointer',
                    fontFamily: 'var(--font-family)',
                    fontSize: 'var(--text-xs)',
                  }}
                >
                  {key}
                  <Badge variant={isExp ? 'warning' : 'success'}>
                    {isExp ? 'Exp' : 'T1'}
                  </Badge>
                  {instanceCount > 0 && (
                    <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-marine-400)' }}>
                      ({instanceCount})
                    </span>
                  )}
                </button>
              );
            })}
            {availableAgents.length === 0 && (
              <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>
                No adapters available
              </span>
            )}
          </div>
          {agentLoadError && (
            <div style={{ marginTop: 'var(--space-1)', color: 'var(--color-warning-400)', fontSize: 'var(--text-xs)' }}>
              {agentLoadError}
            </div>
          )}
        </div>

        {/* Experimental warning */}
        {selectedIsExperimental && (
          <div
            style={{
              padding: 'var(--space-2)',
              borderRadius: 'var(--radius-md)',
              border: '1px solid var(--color-warning-400)',
              backgroundColor: 'color-mix(in srgb, var(--color-warning-400) 8%, transparent)',
            }}
            data-testid="experimental-warning"
          >
            <div style={{ fontSize: 'var(--text-xs)', fontWeight: 'var(--weight-semibold)' as unknown as number, color: 'var(--color-warning-400)', marginBottom: 'var(--space-1)' }}>
              Experimental Adapter Warning
            </div>
            <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-secondary)', marginBottom: 'var(--space-2)' }}>
              This adapter is experimental and may produce unstable results, consume resources unpredictably, or fail without clear error messages. Use at your own risk.
            </div>
            <label style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)', fontSize: 'var(--text-xs)', cursor: 'pointer' }}>
              <input
                type="checkbox"
                checked={experimentalAcknowledged}
                onChange={(e) => setExperimentalAcknowledged(e.target.checked)}
                data-testid="experimental-acknowledge-checkbox"
              />
              <span style={{ color: 'var(--color-text-primary)' }}>I understand the risks</span>
            </label>
          </div>
        )}

        {/* Task prompt */}
        <div>
          <div style={{ marginBottom: 'var(--space-1)', fontSize: 'var(--text-xs)', color: 'var(--color-text-secondary)' }}>
            Task Prompt
          </div>
          <div
            style={{ marginBottom: 'var(--space-1)', fontSize: '10px', color: 'var(--color-text-muted)' }}
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
              padding: 'var(--space-2)',
              resize: 'vertical',
              fontFamily: 'var(--font-family)',
              fontSize: 'var(--text-xs)',
            }}
          />
        </div>

        {/* Launch button */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
          <Button
            variant="primary"
            size="sm"
            onClick={handleConfirmCreate}
            loading={creating}
            disabled={availableAgents.length === 0 || (selectedIsExperimental && !experimentalAcknowledged)}
            data-testid="confirm-create-session"
          >
            Deploy
          </Button>
          {runningInstanceCount > 0 && (
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>
              {runningInstanceCount} running
            </span>
          )}
        </div>

        {/* Create error */}
        {createError && (
          <div
            style={{
              color: createErrorCode === 'dirty_worktree'
                ? 'var(--color-warning-400)'
                : 'var(--color-danger-400)',
              fontSize: 'var(--text-xs)',
              padding: 'var(--space-2)',
              borderRadius: 'var(--radius-md)',
              backgroundColor: createErrorCode
                ? 'color-mix(in srgb, var(--color-danger-400) 8%, transparent)'
                : 'transparent',
            }}
            data-testid="create-session-error"
          >
            {createError}
            {(() => {
              const hint = errorHintForCode(createErrorCode, agentKey);
              return hint ? (
                <div
                  style={{ marginTop: 'var(--space-1)', color: 'var(--color-text-muted)' }}
                  data-testid="create-session-error-hint"
                >
                  {hint}
                </div>
              ) : null;
            })()}
          </div>
        )}
      </div>

      {/* ---- Center: Focused Terminal + Toolbar ---- */}
      <div style={centerStyle}>
        <InteractiveTerminalPanel
          ref={terminalRef}
          sessionId={selectedSessionId}
          agentKey={selectedSession?.agentKey ?? null}
          laneLabel={selectedSession ? laneLabel(selectedSession) : null}
          status={selectedSession?.status ?? null}
          events={selectedEvents}
          transportError={selectedPollError}
          onTerminalInput={selectedSession?.status === 'running' ? handleTerminalInput : undefined}
        />

        {/* P4.9.5: Terminal toolbar — stop + status (replaces InputComposer) */}
        {selectedSessionId && (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 'var(--space-3)',
              padding: 'var(--space-2) var(--space-4)',
              borderTop: '1px solid var(--color-border-700)',
              backgroundColor: 'var(--color-bg-900)',
              flexShrink: 0,
            }}
            data-testid="terminal-toolbar"
          >
            {selectedSession?.status === 'running' && (
              <Button
                variant="danger"
                size="sm"
                onClick={() => {
                  if (selectedSessionId) handleStopSession(selectedSessionId);
                }}
                data-testid="stop-session-btn"
              >
                Stop
              </Button>
            )}
            {selectedSession?.status && selectedSession.status !== 'running' && selectedSession.status !== 'unknown' && (
              <span
                style={{
                  fontSize: 'var(--text-xs)',
                  color: 'var(--color-text-muted)',
                  whiteSpace: 'nowrap',
                }}
                data-testid="session-ended-indicator"
              >
                Session {selectedSession.status}
              </span>
            )}
          </div>
        )}
      </div>

      {/* ---- Right: Running-Lanes Rail ---- */}
      <div style={rightPanelStyle} data-testid="lanes-rail">
        <InteractiveSessionRail
          sessions={sessions}
          selectedSessionId={selectedSessionId}
          pollErrors={pollErrors}
          onSelectSession={(id) => {
            setSelectedSessionId(id);
            const session = sessions.find((entry) => entry.sessionId === id);
            if (!session || session.status === 'running') {
              startPolling(id);
            }
          }}
          onStopSession={handleStopSession}
        />
      </div>
    </div>
  );
}

/** Generate a human-readable lane label for disambiguation (M4.8.2). */
export function laneLabel(session: InteractiveSessionSummary): string {
  return `${session.agentKey} · ${session.sessionId.slice(0, 8)}`;
}
