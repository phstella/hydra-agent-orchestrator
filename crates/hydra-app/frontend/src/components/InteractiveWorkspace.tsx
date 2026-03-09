import { useState, useCallback, useEffect, useMemo, useRef } from 'react';
import type { CSSProperties } from 'react';
import { InteractiveSessionRail } from './InteractiveSessionRail';
import { InteractiveTerminalPanel } from './InteractiveTerminalPanel';
import type { XTermRendererHandle } from './XTermRenderer';
import { Button, Badge } from './design-system';
import {
  startInteractiveSession,
  pollInteractiveEvents,
  listenInteractiveEvents,
  getInteractiveTransportDiagnostics,
  writeInteractiveInput,
  resizeInteractiveTerminal,
  stopInteractiveSession,
  listInteractiveSessions,
  listAdapters,
  type InteractivePushAttachReason,
} from '../ipc';
import type {
  InteractiveSessionSummary,
  InteractiveStreamEvent,
  InteractiveTransportDiagnostics,
  AdapterInfo,
} from '../types';

const MAX_CLIENT_CHUNKS_PER_SESSION = 2_000;
const POLL_INTERVAL_ACTIVE_MS = 40;
const POLL_INTERVAL_BACKGROUND_MS = 120;
const POLL_RETRY_MS = 400;
const STREAM_FLUSH_INTERVAL_MS = 12;
const PUSH_ATTACH_MAX_ATTEMPTS = 3;
const PUSH_ATTACH_RETRY_MS = 350;
const EMPTY_CHUNKS: string[] = [];

type PushAttachDiagnosticReason = 'pending' | InteractivePushAttachReason | 'payload_mismatch';

interface PushAttachDiagnosticState {
  reason: PushAttachDiagnosticReason;
  detail: string | null;
  attempts: number;
  retryScheduled: boolean;
}

function appendBoundedChunk(existing: string[] | undefined, incoming: string): string[] {
  const target = existing ?? [];
  if (incoming.length === 0) return target;
  target.push(incoming);
  if (target.length > MAX_CLIENT_CHUNKS_PER_SESSION) {
    target.splice(0, target.length - MAX_CLIENT_CHUNKS_PER_SESSION);
  }
  return target;
}

function extractTerminalText(event: InteractiveStreamEvent): string {
  if (event.eventType === 'user_input') return '';
  if (event.eventType === 'session_started') return '\r\n--- Session started ---\r\n';
  if (event.eventType === 'session_completed') return '\r\n--- Session completed ---\r\n';
  if (event.eventType === 'session_failed') return '\r\n--- Session failed ---\r\n';
  if (event.eventType === 'session_stopped') return '\r\n--- Session stopped ---\r\n';

  if (typeof event.data === 'string') return event.data;
  const data = asRecord(event.data);
  if (!data) return '';

  if (typeof data.text === 'string') return data.text;
  if (typeof data.line === 'string') {
    const parsed = parseInteractiveStructuredLine(data.line);
    if (parsed) return ensureTerminalLineEnding(parsed);
    return data.line;
  }
  if (typeof data.message === 'string') {
    return ensureTerminalLineEnding(summarizeInteractiveText(data.message));
  }
  if (typeof data.error === 'string') {
    return ensureTerminalLineEnding(`Error: ${summarizeInteractiveText(data.error, 220)}`);
  }

  const humanized = humanizeInteractiveStructuredPayload(data);
  if (humanized) return ensureTerminalLineEnding(humanized);

  return '';
}

function ensureTerminalLineEnding(text: string): string {
  if (!text) return '';
  if (text.endsWith('\n') || text.endsWith('\r')) return text;
  return `${text}\r\n`;
}

function parseInteractiveStructuredLine(line: string): string | null {
  const normalized = normalizeInteractiveTerminalText(line);
  if (!normalized) return '';
  if (!looksLikeJsonValue(normalized)) return summarizeInteractiveText(normalized);

  try {
    const parsed = JSON.parse(normalized) as unknown;
    return humanizeInteractiveStructuredPayload(parsed) ?? summarizeInteractiveText(normalized);
  } catch {
    return summarizeInteractiveText(normalized);
  }
}

function humanizeInteractiveStructuredPayload(payload: unknown): string | null {
  const record = asRecord(payload);
  if (!record) return null;

  if (typeof record.result === 'string') return summarizeInteractiveText(record.result);
  if (typeof record.text === 'string') return summarizeInteractiveText(record.text);
  if (typeof record.message === 'string') return summarizeInteractiveText(record.message);
  if (typeof record.content === 'string') return summarizeInteractiveText(record.content);

  const payloadType = typeof record.type === 'string' ? record.type : null;

  if (payloadType === 'assistant') {
    const message = asRecord(record.message);
    const rendered = renderInteractiveAssistantContent(message);
    if (rendered) return rendered;
  }

  if (payloadType === 'user') {
    const message = asRecord(record.message);
    const rendered = renderInteractiveUserToolResults(record, message);
    if (rendered) return rendered;
  }

  if (payloadType === 'item.completed') {
    const item = asRecord(record.item);
    if (item && typeof item.text === 'string') {
      return summarizeInteractiveText(item.text);
    }
  }

  return null;
}

function renderInteractiveAssistantContent(message: Record<string, unknown> | null): string | null {
  if (!message) return null;
  const content = message.content;
  if (!Array.isArray(content)) return null;

  const lines: string[] = [];
  for (const part of content) {
    const entry = asRecord(part);
    if (!entry) continue;
    if (typeof entry.text === 'string') {
      lines.push(summarizeInteractiveText(entry.text));
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
  return summarizeInteractiveText(lines.join('\n'), 400);
}

function renderInteractiveUserToolResults(
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
        const firstLine = normalizeInteractiveTerminalText(text)
          .split('\n')
          .find((line) => line.trim().length > 0);
        if (firstLine) return summarizeInteractiveText(firstLine, 220);
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

function looksLikeJsonValue(value: string): boolean {
  const trimmed = value.trim();
  if (trimmed.length < 2) return false;
  return (
    (trimmed.startsWith('{') && trimmed.endsWith('}'))
    || (trimmed.startsWith('[') && trimmed.endsWith(']'))
  );
}

function summarizeInteractiveText(raw: string, maxChars = 320): string {
  const normalized = normalizeInteractiveTerminalText(raw).trim();
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

function normalizeInteractiveTerminalText(raw: string): string {
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

/**
 * Drops overlapping/replayed prefix events using cursor math.
 * If a poll batch replays older events, keep only the truly new suffix.
 */
function freshBatchEvents(
  events: InteractiveStreamEvent[],
  cursor: number,
  nextCursor: number,
): InteractiveStreamEvent[] {
  const advanced = Math.max(0, nextCursor - cursor);
  if (advanced >= events.length) return events;
  const overlap = events.length - advanced;
  return events.slice(overlap);
}

type SessionPatch = {
  status: string;
  error: string | null;
};

function sessionPatchFromEvent(event: InteractiveStreamEvent): SessionPatch | null {
  switch (event.eventType) {
    case 'session_started':
      return { status: 'running', error: null };
    case 'session_completed':
      return { status: 'completed', error: null };
    case 'session_stopped':
      return { status: 'stopped', error: null };
    case 'session_failed': {
      const data = event.data && typeof event.data === 'object'
        ? (event.data as Record<string, unknown>)
        : null;
      const error = data && typeof data.error === 'string' ? data.error : null;
      return { status: 'failed', error };
    }
    default:
      return null;
  }
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
  selectedSessionIdOverride?: string | null;
  onSessionSnapshotChange?: (snapshot: InteractiveWorkspaceSessionSnapshot) => void;
}

export interface InteractiveWorkspaceSessionSnapshot {
  sessions: InteractiveSessionSummary[];
  selectedSessionId: string | null;
}

export function InteractiveWorkspace({
  workspaceCwd,
  selectedSessionIdOverride = null,
  onSessionSnapshotChange,
}: InteractiveWorkspaceProps) {
  // ---------------------------------------------------------------------------
  // Session state (all keyed by session_id — M4.8.2)
  // ---------------------------------------------------------------------------
  const [sessions, setSessions] = useState<InteractiveSessionSummary[]>([]);
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [pollErrors, setPollErrors] = useState<Map<string, string>>(new Map());
  const [sessionErrors, setSessionErrors] = useState<Map<string, string>>(new Map());
  const [streamTransport, setStreamTransport] = useState<'pending' | 'push' | 'poll'>('pending');
  const [pushAttachDiagnostics, setPushAttachDiagnostics] = useState<PushAttachDiagnosticState>({
    reason: 'pending',
    detail: null,
    attempts: 0,
    retryScheduled: false,
  });
  const [backendTransportDiagnostics, setBackendTransportDiagnostics] =
    useState<InteractiveTransportDiagnostics | null>(null);

  // ---------------------------------------------------------------------------
  // Create-form state
  // ---------------------------------------------------------------------------
  const [creating, setCreating] = useState(false);
  const [agentKey, setAgentKey] = useState('');
  const [allAdapters, setAllAdapters] = useState<AdapterInfo[]>([]);
  const [availableAgents, setAvailableAgents] = useState<string[]>([]);
  const [agentLoadError, setAgentLoadError] = useState<string | null>(null);
  const [taskPrompt, setTaskPrompt] = useState('');
  const [showInitialPrompt, setShowInitialPrompt] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const [createErrorCode, setCreateErrorCode] = useState<string | null>(null);
  const [allowExperimental, setAllowExperimental] = useState(false);
  const [experimentalAcknowledged, setExperimentalAcknowledged] = useState(false);
  const [unsafeMode, setUnsafeMode] = useState(false);
  const [threadRootInput, setThreadRootInput] = useState(workspaceCwd ?? '');

  // ---------------------------------------------------------------------------
  // Polling refs (session_id keyed — M4.8.2)
  // ---------------------------------------------------------------------------
  const pollTimers = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());
  const pollCursors = useRef<Map<string, number>>(new Map());
  const pollingSessions = useRef<Set<string>>(new Set());
  const terminalSizes = useRef<Map<string, string>>(new Map());
  const streamTransportRef = useRef<'pending' | 'push' | 'poll'>('pending');

  const selectedSessionIdRef = useRef<string | null>(null);
  const sessionChunkStoreRef = useRef<Map<string, string[]>>(new Map());
  const pendingTextBySession = useRef<Map<string, string>>(new Map());
  const pendingPatchBySession = useRef<Map<string, SessionPatch>>(new Map());
  const pendingFlushTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // P4.9.5: Terminal ref for focus management
  const terminalRef = useRef<XTermRendererHandle>(null);

  const flushPendingStreamUpdates = useCallback(() => {
    pendingFlushTimer.current = null;

    const textEntries = [...pendingTextBySession.current.entries()];
    pendingTextBySession.current.clear();

    const patchEntries = [...pendingPatchBySession.current.entries()];
    pendingPatchBySession.current.clear();

    if (textEntries.length > 0) {
      const selectedId = selectedSessionIdRef.current;
      let selectedReplayNeeded = false;
      for (const [sessionId, chunk] of textEntries) {
        const existing = sessionChunkStoreRef.current.get(sessionId) ?? [];
        const next = appendBoundedChunk(existing, chunk);
        sessionChunkStoreRef.current.set(sessionId, next);
        if (sessionId === selectedId) {
          if (terminalRef.current) {
            terminalRef.current.appendChunk(chunk);
          } else {
            selectedReplayNeeded = true;
          }
        }
      }

      if (selectedReplayNeeded && selectedId) {
        setTimeout(() => {
          if (selectedSessionIdRef.current !== selectedId) return;
          const replay = sessionChunkStoreRef.current.get(selectedId) ?? [];
          terminalRef.current?.replaceChunks(replay);
        }, 0);
      }
    }

    if (patchEntries.length > 0) {
      const patchMap = new Map(patchEntries);
      setSessions((prev) => {
        let changed = false;
        const next = prev.map((session) => {
          const patch = patchMap.get(session.sessionId);
          if (!patch || session.status === patch.status) return session;
          changed = true;
          return {
            ...session,
            status: patch.status,
          };
        });
        return changed ? next : prev;
      });

      setSessionErrors((prev) => {
        let changed = false;
        const next = new Map(prev);
        for (const [sessionId, patch] of patchMap.entries()) {
          const current = next.get(sessionId) ?? null;
          if (patch.error) {
            if (current !== patch.error) {
              next.set(sessionId, patch.error);
              changed = true;
            }
          } else if (next.has(sessionId)) {
            next.delete(sessionId);
            changed = true;
          }
        }
        return changed ? next : prev;
      });
    }
  }, []);

  const scheduleStreamFlush = useCallback(() => {
    if (pendingFlushTimer.current) return;
    pendingFlushTimer.current = setTimeout(flushPendingStreamUpdates, STREAM_FLUSH_INTERVAL_MS);
  }, [flushPendingStreamUpdates]);

  const enqueueStreamEvents = useCallback((
    sessionId: string,
    events: InteractiveStreamEvent[],
    fallbackPatch?: SessionPatch,
  ) => {
    if (events.length === 0 && !fallbackPatch) return;

    let appendText = '';
    let latestPatch = fallbackPatch ?? null;

    for (const event of events) {
      const text = extractTerminalText(event);
      if (text.length > 0) {
        appendText += text;
      }
      const patch = sessionPatchFromEvent(event);
      if (patch) {
        latestPatch = patch;
      }
    }

    if (appendText.length > 0) {
      if (sessionId === selectedSessionIdRef.current && terminalRef.current) {
        const existing = sessionChunkStoreRef.current.get(sessionId);
        const next = appendBoundedChunk(existing, appendText);
        sessionChunkStoreRef.current.set(sessionId, next);
        terminalRef.current?.appendChunk(appendText);
      } else {
        const prev = pendingTextBySession.current.get(sessionId) ?? '';
        pendingTextBySession.current.set(sessionId, prev + appendText);
      }
    }
    if (latestPatch) {
      pendingPatchBySession.current.set(sessionId, latestPatch);
    }

    scheduleStreamFlush();
  }, [scheduleStreamFlush]);

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

  useEffect(() => {
    if (workspaceCwd && threadRootInput.trim().length === 0) {
      setThreadRootInput(workspaceCwd);
    }
  }, [workspaceCwd, threadRootInput]);

  // ---------------------------------------------------------------------------
  // Transport selection — prefer push stream in Tauri, fallback to polling.
  // ---------------------------------------------------------------------------
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;
    let retryTimer: ReturnType<typeof setTimeout> | null = null;
    let attempts = 0;

    const clearRetryTimer = () => {
      if (retryTimer) {
        clearTimeout(retryTimer);
        retryTimer = null;
      }
    };

    const clearListener = () => {
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    };

    let attemptAttach: (() => Promise<void>) | null = null;

    const scheduleRetry = (
      reason: PushAttachDiagnosticReason,
      detail: string | null,
    ) => {
      if (attempts >= PUSH_ATTACH_MAX_ATTEMPTS) return;
      const delay = PUSH_ATTACH_RETRY_MS * attempts;
      setPushAttachDiagnostics({
        reason,
        detail,
        attempts,
        retryScheduled: true,
      });
      clearRetryTimer();
      retryTimer = setTimeout(() => {
        retryTimer = null;
        if (attemptAttach) {
          void attemptAttach();
        }
      }, delay);
    };

    const shouldRetryReason = (reason: InteractivePushAttachReason | 'payload_mismatch') =>
      reason === 'listener_error' || reason === 'runtime_block' || reason === 'payload_mismatch';

    const handlePayloadMismatch = (detail: string) => {
      if (cancelled) return;
      clearListener();
      setStreamTransport('poll');
      setPushAttachDiagnostics({
        reason: 'payload_mismatch',
        detail,
        attempts,
        retryScheduled: false,
      });
      if (shouldRetryReason('payload_mismatch')) {
        scheduleRetry('payload_mismatch', detail);
      }
    };

    attemptAttach = async () => {
      if (cancelled || unlisten) return;
      attempts += 1;
      setPushAttachDiagnostics({
        reason: 'pending',
        detail: null,
        attempts,
        retryScheduled: false,
      });

      const attach = await listenInteractiveEvents(
        (event) => {
          enqueueStreamEvents(event.sessionId, [event]);
        },
        handlePayloadMismatch,
      );

      if (cancelled) {
        attach.unlisten?.();
        return;
      }

      if (attach.unlisten) {
        unlisten = attach.unlisten;
        setStreamTransport('push');
        setPushAttachDiagnostics({
          reason: 'attached',
          detail: null,
          attempts,
          retryScheduled: false,
        });
        return;
      }

      setStreamTransport('poll');
      setPushAttachDiagnostics({
        reason: attach.reason,
        detail: attach.detail,
        attempts,
        retryScheduled: false,
      });

      if (shouldRetryReason(attach.reason)) {
        scheduleRetry(attach.reason, attach.detail);
      }
    };

    if (attemptAttach) {
      void attemptAttach();
    }

    return () => {
      cancelled = true;
      clearRetryTimer();
      clearListener();
    };
  }, [enqueueStreamEvents]);

  // ---------------------------------------------------------------------------
  // Pull backend transport diagnostics while in fallback mode (P4.9.7).
  // ---------------------------------------------------------------------------
  useEffect(() => {
    if (!selectedSessionId || streamTransport === 'push') {
      setBackendTransportDiagnostics(null);
      return;
    }

    let cancelled = false;
    let timer: ReturnType<typeof setTimeout> | null = null;

    const refreshDiagnostics = async () => {
      try {
        const diagnostics = await getInteractiveTransportDiagnostics(selectedSessionId);
        if (!cancelled) {
          setBackendTransportDiagnostics(diagnostics);
        }
      } catch {
        if (!cancelled) {
          setBackendTransportDiagnostics(null);
        }
      } finally {
        if (!cancelled) {
          timer = setTimeout(refreshDiagnostics, 1500);
        }
      }
    };

    void refreshDiagnostics();

    return () => {
      cancelled = true;
      if (timer) clearTimeout(timer);
    };
  }, [selectedSessionId, streamTransport]);

  // ---------------------------------------------------------------------------
  // Polling — per-session isolated (M4.8.5)
  // ---------------------------------------------------------------------------
  const startPolling = useCallback((sessionId: string) => {
    if (streamTransportRef.current === 'push') return;
    if (pollingSessions.current.has(sessionId)) return;
    pollingSessions.current.add(sessionId);
    if (!pollCursors.current.has(sessionId)) {
      pollCursors.current.set(sessionId, 0);
    }

    function poll() {
      if (streamTransportRef.current === 'push') {
        const timer = pollTimers.current.get(sessionId);
        if (timer) clearTimeout(timer);
        pollTimers.current.delete(sessionId);
        pollingSessions.current.delete(sessionId);
        return;
      }

      const cursor = pollCursors.current.get(sessionId) ?? 0;

      pollInteractiveEvents(sessionId, cursor)
        .then((batch) => {
          setPollErrors((prev) => {
            if (!prev.has(sessionId)) return prev;
            const next = new Map(prev);
            next.delete(sessionId);
            return next;
          });

          // Keep cursor monotonic client-side to avoid replay loops.
          const nextCursor = Math.max(cursor, batch.nextCursor);
          pollCursors.current.set(sessionId, nextCursor);

          const freshEvents = freshBatchEvents(batch.events, cursor, nextCursor);
          enqueueStreamEvents(sessionId, freshEvents, {
            status: batch.status,
            error: batch.error ?? null,
          });

          if (batch.done) {
            pollTimers.current.delete(sessionId);
            pollingSessions.current.delete(sessionId);
            return;
          }

          const pollDelay = selectedSessionIdRef.current === sessionId
            ? POLL_INTERVAL_ACTIVE_MS
            : POLL_INTERVAL_BACKGROUND_MS;
          const timer = setTimeout(poll, pollDelay);
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
    streamTransportRef.current = streamTransport;
    if (streamTransport === 'push') {
      pollTimers.current.forEach((timer) => clearTimeout(timer));
      pollTimers.current.clear();
      pollingSessions.current.clear();
      setPollErrors(new Map());
    }
  }, [streamTransport]);

  useEffect(() => {
    if (streamTransport === 'push') return;

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
  }, [sessions, startPolling, streamTransport]);

  // ---------------------------------------------------------------------------
  // Session creation — supports duplicate adapter instances (M4.8.3)
  // ---------------------------------------------------------------------------
  const selectedAdapterInfo = allAdapters.find((a) => a.key === agentKey) ?? null;
  const selectedIsExperimental = selectedAdapterInfo?.tier === 'experimental';
  const needsExperimentalConfirmation = selectedIsExperimental && !experimentalAcknowledged;
  const showPromptComposer = sessions.length === 0 || showInitialPrompt;

  const handleConfirmCreate = useCallback(async () => {
    if (!agentKey) {
      setCreateError('Select an available agent first.');
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
      const initialPrompt = showPromptComposer ? taskPrompt.trim() : '';
      const targetThreadRoot = threadRootInput.trim().length > 0
        ? threadRootInput.trim()
        : (workspaceCwd ?? null);
      const result = await startInteractiveSession({
        agentKey,
        taskPrompt: initialPrompt,
        allowExperimental: allowExperimental && experimentalAcknowledged,
        unsafeMode,
        cwd: targetThreadRoot,
        cols: 120,
        rows: 30,
      });

      const newSession: InteractiveSessionSummary = {
        sessionId: result.sessionId,
        agentKey: result.agentKey,
        status: result.status,
        startedAt: result.startedAt,
        eventCount: 0,
        sourceRoot: result.sourceRoot,
        repoRoot: result.repoRoot,
        effectiveCwd: result.effectiveCwd,
        worktreePath: result.worktreePath,
      };

      setSessions((prev) => [newSession, ...prev]);
      selectedSessionIdRef.current = result.sessionId;
      setSelectedSessionId(result.sessionId);
      setTaskPrompt('');
      setShowInitialPrompt(false);
      setUnsafeMode(false);
      setAllowExperimental(false);
      setExperimentalAcknowledged(false);
      if (streamTransport !== 'push') {
        startPolling(result.sessionId);
      }
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setCreateError(errorMessage);
      setCreateErrorCode(parseGatingErrorCode(errorMessage));
    } finally {
      setCreating(false);
    }
  }, [
    agentKey,
    taskPrompt,
    showPromptComposer,
    startPolling,
    allowExperimental,
    experimentalAcknowledged,
    unsafeMode,
    streamTransport,
    needsExperimentalConfirmation,
    threadRootInput,
    workspaceCwd,
  ]);

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
      setSessionErrors((prev) => {
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
      terminalSizes.current.delete(sessionId);
      sessionChunkStoreRef.current.delete(sessionId);
      if (selectedSessionIdRef.current === sessionId) {
        terminalRef.current?.replaceChunks([]);
      }
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

  const handleTerminalResize = useCallback(
    (cols: number, rows: number) => {
      if (!selectedSessionId) return;
      const dimsKey = `${cols}x${rows}`;
      if (terminalSizes.current.get(selectedSessionId) === dimsKey) return;
      terminalSizes.current.set(selectedSessionId, dimsKey);
      resizeInteractiveTerminal(selectedSessionId, cols, rows).catch(() => {
        // best-effort; resize failures are non-fatal
      });
    },
    [selectedSessionId],
  );

  useEffect(() => {
    selectedSessionIdRef.current = selectedSessionId;
    requestAnimationFrame(() => {
      const replay = selectedSessionId
        ? (sessionChunkStoreRef.current.get(selectedSessionId) ?? [])
        : [];
      terminalRef.current?.replaceChunks(replay);
    });
  }, [selectedSessionId]);

  useEffect(() => {
    if (!selectedSessionIdOverride) return;
    if (selectedSessionIdRef.current === selectedSessionIdOverride) return;
    const session = sessions.find((entry) => entry.sessionId === selectedSessionIdOverride);
    if (!session) return;
    selectedSessionIdRef.current = selectedSessionIdOverride;
    setSelectedSessionId(selectedSessionIdOverride);
    if (session.status === 'running') {
      startPolling(selectedSessionIdOverride);
    }
  }, [selectedSessionIdOverride, sessions, startPolling]);

  useEffect(() => {
    onSessionSnapshotChange?.({ sessions, selectedSessionId });
  }, [onSessionSnapshotChange, sessions, selectedSessionId]);

  // ---------------------------------------------------------------------------
  // Derived state for focused lane
  // ---------------------------------------------------------------------------
  const selectedSession = sessions.find((s) => s.sessionId === selectedSessionId) ?? null;
  const selectedPollError = selectedSessionId
    ? (pollErrors.get(selectedSessionId) ?? null)
    : null;
  const selectedSessionError = selectedSessionId
    ? (sessionErrors.get(selectedSessionId) ?? null)
    : null;

  const laneErrors = useMemo(() => {
    const merged = new Map(pollErrors);
    sessionErrors.forEach((error, sessionId) => {
      if (!merged.has(sessionId)) {
        merged.set(sessionId, error);
      }
    });
    return merged;
  }, [pollErrors, sessionErrors]);

  // Count duplicate adapter instances for lane label disambiguation (M4.8.2)
  const runningInstanceCount = agentKey ? countAdapterInstances(sessions, agentKey) : 0;
  const reduceRailMotion = streamTransport === 'push' || sessions.some((session) => session.status === 'running');
  const transportDiagnostic = useMemo(() => {
    const details: string[] = [];
    const pushEmitErrorCount = backendTransportDiagnostics?.pushEmitErrorCount ?? 0;
    const attachReason = pushAttachDiagnostics.reason;
    const showAttachReason = attachReason === 'runtime_block'
      || attachReason === 'payload_mismatch';
    const showRetryDetail = pushAttachDiagnostics.retryScheduled
      && (attachReason === 'runtime_block' || attachReason === 'payload_mismatch');

    if (showAttachReason) {
      details.push(`attach:${attachReason}`);
    }
    if (showRetryDetail) {
      details.push(`retry ${pushAttachDiagnostics.attempts}/${PUSH_ATTACH_MAX_ATTEMPTS}`);
    }
    if (pushEmitErrorCount > 0) {
      details.push(`emit errors:${pushEmitErrorCount}`);
    }
    if (details.length === 0) return null;
    return details.join(' · ');
  }, [
    backendTransportDiagnostics?.pushEmitErrorCount,
    pushAttachDiagnostics.attempts,
    pushAttachDiagnostics.reason,
    pushAttachDiagnostics.retryScheduled,
  ]);
  const transportDiagnosticDetail = useMemo(() => {
    const details: string[] = [];
    if (pushAttachDiagnostics.detail) {
      details.push(pushAttachDiagnostics.detail);
    }
    if (backendTransportDiagnostics?.lastPushEmitError) {
      details.push(`backend emit: ${backendTransportDiagnostics.lastPushEmitError}`);
    }
    if (details.length === 0) return null;
    return details.join(' | ');
  }, [pushAttachDiagnostics.detail, backendTransportDiagnostics?.lastPushEmitError]);

  // ---------------------------------------------------------------------------
  // Cleanup on unmount
  // ---------------------------------------------------------------------------
  useEffect(() => {
    return () => {
      pollTimers.current.forEach((timer) => clearTimeout(timer));
      pollTimers.current.clear();
      pollCursors.current.clear();
      pollingSessions.current.clear();
      terminalSizes.current.clear();
      sessionChunkStoreRef.current.clear();
      pendingTextBySession.current.clear();
      pendingPatchBySession.current.clear();
      if (pendingFlushTimer.current) {
        clearTimeout(pendingFlushTimer.current);
        pendingFlushTimer.current = null;
      }
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
          New Thread
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

        {/* Workspace + initial prompt controls */}
        <div>
          <div
            style={{ marginBottom: 'var(--space-1)', fontSize: '10px', color: 'var(--color-text-muted)' }}
            data-testid="interactive-workspace-path"
          >
            Thread Folder
          </div>
          <div style={{ display: 'flex', gap: 'var(--space-2)', alignItems: 'center', marginBottom: 'var(--space-2)' }}>
            <input
              type="text"
              value={threadRootInput}
              onChange={(e) => setThreadRootInput(e.target.value)}
              placeholder={workspaceCwd ?? '/path/to/project'}
              data-testid="thread-root-input"
              style={{
                width: '100%',
                borderRadius: 'var(--radius-md)',
                border: '1px solid var(--color-border-700)',
                backgroundColor: 'var(--color-bg-900)',
                color: 'var(--color-text-primary)',
                padding: 'var(--space-1) var(--space-2)',
                fontFamily: 'var(--font-mono)',
                fontSize: 'var(--text-xs)',
              }}
            />
            <button
              type="button"
              onClick={() => setThreadRootInput(workspaceCwd ?? '')}
              data-testid="thread-root-use-workspace"
              style={{
                border: '1px solid var(--color-border-700)',
                backgroundColor: 'var(--color-surface-800)',
                color: 'var(--color-text-secondary)',
                borderRadius: 'var(--radius-md)',
                padding: 'var(--space-1) var(--space-2)',
                cursor: 'pointer',
                fontSize: 'var(--text-xs)',
                whiteSpace: 'nowrap',
              }}
            >
              Use Workspace
            </button>
          </div>
          <div style={{ marginBottom: 'var(--space-1)', fontSize: '10px', color: 'var(--color-text-muted)' }}>
            Default workspace: {workspaceCwd ?? '(current repository)'}
          </div>
          <div style={{ marginBottom: 'var(--space-1)', fontSize: '10px', color: 'var(--color-text-muted)' }}>
            Deploy creates the thread. Send normal prompts directly in the terminal.
          </div>
          {!showPromptComposer && (
            <button
              type="button"
              onClick={() => setShowInitialPrompt(true)}
              data-testid="show-initial-prompt-btn"
              style={{
                border: '1px solid var(--color-border-700)',
                backgroundColor: 'var(--color-surface-800)',
                color: 'var(--color-text-secondary)',
                borderRadius: 'var(--radius-md)',
                padding: 'var(--space-1) var(--space-2)',
                cursor: 'pointer',
                fontSize: 'var(--text-xs)',
              }}
            >
              Add Initial Prompt (Optional)
            </button>
          )}
          {showPromptComposer && (
            <>
              <div style={{ marginTop: 'var(--space-2)', marginBottom: 'var(--space-1)', fontSize: 'var(--text-xs)', color: 'var(--color-text-secondary)' }}>
                Initial Prompt (optional)
              </div>
              <textarea
                value={taskPrompt}
                onChange={(e) => setTaskPrompt(e.target.value)}
                placeholder="Optional one-time bootstrap prompt..."
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
            </>
          )}
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
          streamTransport={streamTransport}
          transportDiagnostic={transportDiagnostic}
          transportDiagnosticDetail={transportDiagnosticDetail}
          chunks={EMPTY_CHUNKS}
          transportError={selectedPollError}
          sessionError={selectedSessionError}
          onTerminalInput={selectedSession?.status === 'running' ? handleTerminalInput : undefined}
          onTerminalResize={selectedSession?.status === 'running' ? handleTerminalResize : undefined}
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
          pollErrors={laneErrors}
          reduceMotion={reduceRailMotion}
          onSelectSession={(id) => {
            selectedSessionIdRef.current = id;
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
