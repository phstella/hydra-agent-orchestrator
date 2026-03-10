/**
 * Tauri IPC bridge.
 *
 * When running outside Tauri (e.g. `npm run dev` standalone), falls back
 * to mock data so the UI can be developed without the Rust backend.
 */

import type {
  PreflightResult,
  AdapterInfo,
  AgentStreamEvent,
  RaceRequest,
  RaceStarted,
  RaceResult,
  RaceEventBatch,
  WorkingTreeStatus,
  CandidateDiffPayload,
  MergePreviewPayload,
  MergeExecutionPayload,
  InteractiveSessionRequest,
  InteractiveSessionStarted,
  InteractiveEventBatch,
  InteractiveStreamEvent,
  InteractiveWriteAck,
  InteractiveResizeAck,
  InteractiveStopResult,
  InteractiveRemoveResult,
  InteractiveSessionSummary,
  InteractiveTransportDiagnostics,
  DirectoryListing,
  FilePreview,
  FileWatcherStarted,
  FileWatchEventBatch,
  FileWatcherStopped,
} from './types';

type InvokeFn = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
type UnlistenFn = () => void;
const INTERACTIVE_STREAM_EVENT = 'hydra://interactive-event';

export type InteractivePushAttachReason =
  | 'attached'
  | 'unavailable_api'
  | 'listener_error'
  | 'runtime_block';

export interface InteractivePushAttachResult {
  unlisten: UnlistenFn | null;
  reason: InteractivePushAttachReason;
  detail: string | null;
}

let _invoke: InvokeFn | null = null;

async function getInvoke(): Promise<InvokeFn> {
  if (_invoke) return _invoke;

  try {
    const mod = await import('@tauri-apps/api/core');
    _invoke = mod.invoke as InvokeFn;
  } catch {
    if (import.meta.env.VITE_ALLOW_MOCK_IPC === 'true') {
      _invoke = mockInvoke;
    } else {
      throw new Error(
        'Tauri IPC is unavailable. Run inside Tauri, or set VITE_ALLOW_MOCK_IPC=true for standalone mock mode.',
      );
    }
  }
  return _invoke;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export async function healthCheck(): Promise<{ status: string; version: string }> {
  const invoke = await getInvoke();
  return invoke('health_check');
}

export async function runPreflight(): Promise<PreflightResult> {
  const invoke = await getInvoke();
  return invoke('run_preflight');
}

export async function listAdapters(): Promise<AdapterInfo[]> {
  const invoke = await getInvoke();
  return invoke('list_adapters');
}

export async function startRace(request: RaceRequest): Promise<RaceStarted> {
  const invoke = await getInvoke();
  return invoke('start_race', { request });
}

export async function getRaceResult(runId: string): Promise<RaceResult | null> {
  const invoke = await getInvoke();
  return invoke('get_race_result', { runId });
}

export async function pollRaceEvents(runId: string, cursor: number): Promise<RaceEventBatch> {
  const invoke = await getInvoke();
  return invoke('poll_race_events', { runId, cursor });
}

export async function getCandidateDiff(
  runId: string,
  agentKey: string,
  cwd?: string | null,
): Promise<CandidateDiffPayload> {
  const invoke = await getInvoke();
  return invoke('get_candidate_diff', { runId, agentKey, cwd: cwd ?? null });
}

export async function getWorkingTreeStatus(cwd?: string | null): Promise<WorkingTreeStatus> {
  const invoke = await getInvoke();
  return invoke('get_working_tree_status', { cwd: cwd ?? null });
}

export async function previewMerge(
  runId: string,
  agentKey: string,
  force: boolean,
  cwd?: string | null,
): Promise<MergePreviewPayload> {
  const invoke = await getInvoke();
  return invoke('preview_merge', { runId, agentKey, force, cwd: cwd ?? null });
}

export async function executeMerge(
  runId: string,
  agentKey: string,
  force: boolean,
  cwd?: string | null,
): Promise<MergeExecutionPayload> {
  const invoke = await getInvoke();
  return invoke('execute_merge', { runId, agentKey, force, cwd: cwd ?? null });
}

// ---------------------------------------------------------------------------
// Interactive Session API (M4.3 / M4.4)
// ---------------------------------------------------------------------------

export async function startInteractiveSession(
  request: InteractiveSessionRequest,
): Promise<InteractiveSessionStarted> {
  const invoke = await getInvoke();
  return invoke('start_interactive_session', { request });
}

export async function pollInteractiveEvents(
  sessionId: string,
  cursor: number,
): Promise<InteractiveEventBatch> {
  const invoke = await getInvoke();
  return invoke('poll_interactive_events', { sessionId, cursor });
}

export async function writeInteractiveInput(
  sessionId: string,
  input: string,
): Promise<InteractiveWriteAck> {
  const invoke = await getInvoke();
  return invoke('write_interactive_input', { sessionId, input });
}

export async function resizeInteractiveTerminal(
  sessionId: string,
  cols: number,
  rows: number,
): Promise<InteractiveResizeAck> {
  const invoke = await getInvoke();
  return invoke('resize_interactive_terminal', { sessionId, cols, rows });
}

export async function stopInteractiveSession(
  sessionId: string,
): Promise<InteractiveStopResult> {
  const invoke = await getInvoke();
  return invoke('stop_interactive_session', { sessionId });
}

export async function removeInteractiveSession(
  sessionId: string,
): Promise<InteractiveRemoveResult> {
  const invoke = await getInvoke();
  return invoke('remove_interactive_session', { sessionId });
}

export async function listInteractiveSessions(): Promise<InteractiveSessionSummary[]> {
  const invoke = await getInvoke();
  return invoke('list_interactive_sessions');
}

export async function getInteractiveTransportDiagnostics(
  sessionId: string,
): Promise<InteractiveTransportDiagnostics> {
  const invoke = await getInvoke();
  return invoke('get_interactive_transport_diagnostics', { sessionId });
}

/**
 * Prefer push-stream interactive events when running inside Tauri.
 * Returns attach diagnostics so callers can decide whether to retry or
 * fallback to polling.
 */
function normalizeErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error ?? 'unknown error');
}

function classifyInteractiveListenFailure(
  error: unknown,
): { reason: 'listener_error' | 'runtime_block'; detail: string } {
  const detail = normalizeErrorMessage(error);
  const lower = detail.toLowerCase();
  if (
    lower.includes('permission')
    || lower.includes('denied')
    || lower.includes('forbidden')
    || lower.includes('blocked')
    || lower.includes('security')
  ) {
    return { reason: 'runtime_block', detail };
  }
  return { reason: 'listener_error', detail };
}

function isInteractiveStreamEventPayload(payload: unknown): payload is InteractiveStreamEvent {
  if (!payload || typeof payload !== 'object') return false;
  const candidate = payload as Record<string, unknown>;
  return typeof candidate.sessionId === 'string'
    && typeof candidate.agentKey === 'string'
    && typeof candidate.eventType === 'string'
    && typeof candidate.timestamp === 'string';
}

export async function listenInteractiveEvents(
  onEvent: (event: InteractiveStreamEvent) => void,
  onPayloadMismatch?: (detail: string) => void,
): Promise<InteractivePushAttachResult> {
  try {
    const mod = await import('@tauri-apps/api/event');
    const unlisten = await mod.listen<unknown>(INTERACTIVE_STREAM_EVENT, (event) => {
      if (isInteractiveStreamEventPayload(event.payload)) {
        onEvent(event.payload);
        return;
      }
      const payloadType = event.payload === null ? 'null' : typeof event.payload;
      onPayloadMismatch?.(`invalid interactive-event payload type: ${payloadType}`);
    });
    return { unlisten, reason: 'attached', detail: null };
  } catch (error) {
    const message = normalizeErrorMessage(error);
    const lower = message.toLowerCase();
    if (
      lower.includes('@tauri-apps/api/event')
      || lower.includes('tauri ipc is unavailable')
      || lower.includes('failed to fetch dynamically imported module')
      || lower.includes('cannot find module')
    ) {
      return {
        unlisten: null,
        reason: 'unavailable_api',
        detail: message,
      };
    }
    const classified = classifyInteractiveListenFailure(error);
    return {
      unlisten: null,
      reason: classified.reason,
      detail: classified.detail,
    };
  }
}

// ---------------------------------------------------------------------------
// File Explorer API (P4.9.2)
// ---------------------------------------------------------------------------

export async function listDirectory(path: string): Promise<DirectoryListing> {
  const invoke = await getInvoke();
  return invoke('list_directory', { path });
}

export async function readFilePreview(
  path: string,
  maxBytes?: number,
): Promise<FilePreview> {
  const invoke = await getInvoke();
  return invoke('read_file_preview', { path, maxBytes: maxBytes ?? null });
}

export async function startFileWatcher(root: string): Promise<FileWatcherStarted> {
  const invoke = await getInvoke();
  return invoke('start_file_watcher', { root });
}

export async function pollFileWatchEvents(
  watcherId: string,
  cursor: number,
): Promise<FileWatchEventBatch> {
  const invoke = await getInvoke();
  return invoke('poll_file_watch_events', { watcherId, cursor });
}

export async function stopFileWatcher(watcherId: string): Promise<FileWatcherStopped> {
  const invoke = await getInvoke();
  return invoke('stop_file_watcher', { watcherId });
}

// ---------------------------------------------------------------------------
// Mock fallback for standalone frontend dev
// ---------------------------------------------------------------------------

const MOCK_ADAPTERS: AdapterInfo[] = [
  {
    key: 'claude',
    tier: 'tier1',
    status: 'ready',
    version: '1.0.22',
    confidence: 'verified',
    capabilities: {
      json_stream: { supported: true, confidence: 'verified' },
      plain_text: { supported: true, confidence: 'verified' },
      force_edit_mode: { supported: true, confidence: 'verified' },
      sandbox_controls: { supported: false, confidence: 'unknown' },
      approval_controls: { supported: false, confidence: 'unknown' },
      session_resume: { supported: false, confidence: 'unknown' },
      emits_usage: { supported: true, confidence: 'verified' },
    },
  },
  {
    key: 'codex',
    tier: 'tier1',
    status: 'ready',
    version: '0.1.2025061301',
    confidence: 'verified',
    capabilities: {
      json_stream: { supported: true, confidence: 'verified' },
      plain_text: { supported: true, confidence: 'verified' },
      force_edit_mode: { supported: false, confidence: 'observed' },
      sandbox_controls: { supported: true, confidence: 'verified' },
      approval_controls: { supported: true, confidence: 'verified' },
      session_resume: { supported: false, confidence: 'unknown' },
      emits_usage: { supported: true, confidence: 'verified' },
    },
  },
  {
    key: 'cursor-agent',
    tier: 'experimental',
    status: 'experimental_ready',
    version: null,
    confidence: 'observed',
    capabilities: {
      json_stream: { supported: false, confidence: 'unknown' },
      plain_text: { supported: true, confidence: 'observed' },
      force_edit_mode: { supported: false, confidence: 'unknown' },
      sandbox_controls: { supported: false, confidence: 'unknown' },
      approval_controls: { supported: false, confidence: 'unknown' },
      session_resume: { supported: false, confidence: 'unknown' },
      emits_usage: { supported: false, confidence: 'unknown' },
    },
  },
];

const MOCK_PREFLIGHT: PreflightResult = {
  systemReady: true,
  allTier1Ready: true,
  passedCount: 4,
  failedCount: 0,
  totalCount: 4,
  healthScore: 100,
  checks: [
    { name: 'Environment Variables Check', description: 'Found system configuration', status: 'passed', evidence: null },
    { name: 'Checking Git Repository', description: 'Clean working tree on current branch', status: 'passed', evidence: null },
    { name: 'Validating Adapters', description: 'Connected to Anthropic and OpenAI endpoints', status: 'passed', evidence: 'Connected to 2 adapter(s)' },
    { name: 'Running Startup Smoke Tests', description: 'Latency checks: OK. Token estimation: OK.', status: 'passed', evidence: null },
  ],
  adapters: MOCK_ADAPTERS,
  warnings: [],
};

let mockCursor = 0;
let mockStartTime = Date.now();

function mockTs(offsetMs: number): string {
  return new Date(mockStartTime + offsetMs).toISOString();
}

function buildMockEventStream(): AgentStreamEvent[] {
  return [
    { runId: 'mock-run', agentKey: 'system', eventType: 'race_process_started', data: {}, timestamp: mockTs(0) },
    { runId: 'mock-run', agentKey: 'claude', eventType: 'agent_started', data: {}, timestamp: mockTs(100) },
    { runId: 'mock-run', agentKey: 'codex', eventType: 'agent_started', data: {}, timestamp: mockTs(150) },
    { runId: 'mock-run', agentKey: 'claude', eventType: 'agent_stdout', data: { line: 'Analyzing repository structure...' }, timestamp: mockTs(800) },
    { runId: 'mock-run', agentKey: 'codex', eventType: 'agent_stdout', data: { line: 'Reading project configuration...' }, timestamp: mockTs(900) },
    { runId: 'mock-run', agentKey: 'claude', eventType: 'agent_stdout', data: { line: 'Scanning source files for context...' }, timestamp: mockTs(1500) },
    { runId: 'mock-run', agentKey: 'codex', eventType: 'agent_stdout', data: { line: 'Building dependency graph...' }, timestamp: mockTs(1700) },
    { runId: 'mock-run', agentKey: 'claude', eventType: 'agent_stdout', data: { line: 'Generating implementation plan...' }, timestamp: mockTs(2200) },
    { runId: 'mock-run', agentKey: 'codex', eventType: 'agent_stdout', data: { line: 'Applying changes to src/main.rs...' }, timestamp: mockTs(2500) },
    { runId: 'mock-run', agentKey: 'claude', eventType: 'agent_stdout', data: { line: 'Writing changes to 3 files...' }, timestamp: mockTs(3000) },
    { runId: 'mock-run', agentKey: 'codex', eventType: 'agent_stdout', data: { line: 'Running tests...' }, timestamp: mockTs(3200) },
    { runId: 'mock-run', agentKey: 'claude', eventType: 'agent_stdout', data: { line: 'Running cargo test...' }, timestamp: mockTs(3500) },
    { runId: 'mock-run', agentKey: 'codex', eventType: 'agent_stdout', data: { line: 'All tests passed (12/12)' }, timestamp: mockTs(4000) },
    { runId: 'mock-run', agentKey: 'claude', eventType: 'agent_stdout', data: { line: '14 tests passed, 0 failed' }, timestamp: mockTs(4200) },
    { runId: 'mock-run', agentKey: 'codex', eventType: 'agent_completed', data: { durationMs: 4100 }, timestamp: mockTs(4300) },
    { runId: 'mock-run', agentKey: 'claude', eventType: 'agent_completed', data: { durationMs: 4500 }, timestamp: mockTs(4600) },
    { runId: 'mock-run', agentKey: 'system', eventType: 'race_completed', data: {}, timestamp: mockTs(4700) },
  ];
}

let mockEventStream = buildMockEventStream();

const MOCK_DIFF_CLAUDE = `diff --git a/src/main.rs b/src/main.rs
index abc1234..def5678 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -10,6 +10,18 @@ fn main() {
     let config = load_config();
-    println!("starting");
-    println!("done");
+    tracing::info!("starting application");
+    let result = run_pipeline(&config);
+    match result {
+        Ok(()) => tracing::info!("pipeline completed"),
+        Err(e) => {
+            tracing::error!(error = %e, "pipeline failed");
+            std::process::exit(1);
+        }
+    }
+}
+
+fn run_pipeline(config: &Config) -> Result<()> {
+    validate_inputs(config)?;
+    execute_stages(config)?;
+    Ok(())
 }
diff --git a/src/lib.rs b/src/lib.rs
index 1234567..abcdef0 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,8 +1,22 @@
-pub fn process() {
-    // TODO: implement
+pub fn process(input: &str) -> Result<Output, ProcessError> {
+    let parsed = parse_input(input)?;
+    let validated = validate(parsed)?;
+    transform(validated)
 }
diff --git a/tests/integration.rs b/tests/integration.rs
index aaa1111..bbb2222 100644
--- a/tests/integration.rs
+++ b/tests/integration.rs
@@ -5,1 +5,5 @@
-    assert!(true);
+    let result = process("test input");
+    assert!(result.is_ok());
+    let output = result.unwrap();
+    assert_eq!(output.status, "success");
+    assert!(!output.data.is_empty());
`;

const MOCK_DIFF_CODEX = `diff --git a/src/main.rs b/src/main.rs
index abc1234..def5678 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,30 @@
+use anyhow::Result;
+use tracing_subscriber;
+
 fn main() {
+    tracing_subscriber::init();
     let config = load_config();
-    println!("starting");
+    if let Err(e) = run(config) {
+        eprintln!("Error: {e:#}");
+        std::process::exit(1);
+    }
+}
+
+fn run(config: Config) -> Result<()> {
+    let ctx = Context::new(config)?;
+    ctx.execute()?;
+    Ok(())
 }
diff --git a/src/utils.rs b/src/utils.rs
index 1111111..2222222 100644
--- a/src/utils.rs
+++ b/src/utils.rs
@@ -1,4 +1,18 @@
-pub fn helper() {}
+pub fn helper(input: &str) -> String {
+    input.trim().to_lowercase()
+}
+
+pub fn validate_path(path: &std::path::Path) -> bool {
+    path.exists() && path.is_file()
+}
`;

interface MockInteractiveSession {
  sessionId: string;
  agentKey: string;
  status: string;
  startedAt: string;
  sourceRoot: string;
  repoRoot: string;
  effectiveCwd: string;
  worktreePath: string | null;
  eventCursor: number;
  events: Array<{
    sessionId: string;
    agentKey: string;
    eventType: string;
    data: unknown;
    timestamp: string;
  }>;
}

const mockInteractiveSessions = new Map<string, MockInteractiveSession>();

function buildMockInteractiveEvents(
  sessionId: string,
  agentKey: string,
): MockInteractiveSession['events'] {
  const now = Date.now();
  return [
    { sessionId, agentKey, eventType: 'session_started', data: {}, timestamp: new Date(now).toISOString() },
    { sessionId, agentKey, eventType: 'pty_output', data: { text: `[${agentKey}] Initializing interactive session...\n` }, timestamp: new Date(now + 200).toISOString() },
    { sessionId, agentKey, eventType: 'pty_output', data: { text: `[${agentKey}] Ready for input.\n` }, timestamp: new Date(now + 500).toISOString() },
  ];
}

async function mockInvoke<T>(cmd: string, _args?: Record<string, unknown>): Promise<T> {
  await new Promise((r) => setTimeout(r, 200 + Math.random() * 300));

  switch (cmd) {
    case 'health_check':
      return { status: 'ok', version: '0.1.0' } as T;
    case 'run_preflight':
      return MOCK_PREFLIGHT as T;
    case 'list_adapters':
      return MOCK_ADAPTERS as T;
    case 'start_race':
      mockCursor = 0;
      mockStartTime = Date.now();
      mockEventStream = buildMockEventStream();
      return { runId: 'mock-run', agents: ['claude', 'codex'] } as T;
    case 'get_race_result':
      return {
        runId: 'mock-run',
        status: 'completed',
        durationMs: 4700,
        totalCost: 0.42,
        agents: [
          {
            agentKey: 'claude',
            status: 'completed',
            durationMs: 4500,
            score: 93.2,
            mergeable: true,
            gateFailures: [],
            dimensions: [
              { name: 'build', score: 100.0, evidence: { exit_code: 0 } },
              { name: 'tests', score: 92.0, evidence: { passed: 14, failed: 0, baseline_passed: 14, regression: 0, new_tests: 1 } },
              { name: 'lint', score: 95.0, evidence: { baseline_warnings: 3, current_warnings: 2 } },
              { name: 'diff_scope', score: 78.0, evidence: { files_changed: 3, lines_added: 42, lines_removed: 12 } },
              { name: 'speed', score: 100.0, evidence: { agent_duration_ms: 4500, fastest_ms: 4500 } },
            ],
          },
          {
            agentKey: 'codex',
            status: 'completed',
            durationMs: 4100,
            score: 88.5,
            mergeable: true,
            gateFailures: [],
            dimensions: [
              { name: 'build', score: 100.0, evidence: { exit_code: 0 } },
              { name: 'tests', score: 85.0, evidence: { passed: 12, failed: 0, baseline_passed: 14, regression: 2, new_tests: 0 } },
              { name: 'lint', score: 88.0, evidence: { baseline_warnings: 3, current_warnings: 4 } },
              { name: 'diff_scope', score: 72.0, evidence: { files_changed: 5, lines_added: 68, lines_removed: 18 } },
              { name: 'speed', score: 100.0, evidence: { agent_duration_ms: 4100, fastest_ms: 4100 } },
            ],
          },
        ],
      } as T;
    case 'poll_race_events': {
      const elapsed = Date.now() - mockStartTime;
      const available = mockEventStream.filter((e) => {
        const offset = new Date(e.timestamp).getTime() - mockStartTime;
        return offset <= elapsed;
      });
      const batch = available.slice(mockCursor, mockCursor + 3);
      mockCursor += batch.length;
      const done = mockCursor >= mockEventStream.length && batch.length > 0
        && batch.some((e) => e.eventType === 'race_completed');
      return {
        runId: 'mock-run',
        events: batch,
        nextCursor: mockCursor,
        done,
        status: done ? 'completed' : 'running',
        error: null,
      } as T;
    }
    case 'get_candidate_diff': {
      const args = _args as Record<string, unknown> | undefined;
      const agentKey = (args?.agentKey as string) ?? 'claude';
      const isClaude = agentKey === 'claude';
      return {
        runId: 'mock-run',
        agentKey,
        baseRef: 'HEAD~1',
        branch: `hydra/mock-run/agent/${agentKey}`,
        mergeable: isClaude ? true : true,
        gateFailures: [],
        diffText: isClaude ? MOCK_DIFF_CLAUDE : MOCK_DIFF_CODEX,
        files: isClaude
          ? [
              { path: 'src/main.rs', added: 15, removed: 3 },
              { path: 'src/lib.rs', added: 22, removed: 8 },
              { path: 'tests/integration.rs', added: 5, removed: 1 },
            ]
          : [
              { path: 'src/main.rs', added: 30, removed: 5 },
              { path: 'src/utils.rs', added: 18, removed: 4 },
              { path: 'src/config.rs', added: 12, removed: 6 },
              { path: 'tests/unit.rs', added: 8, removed: 3 },
            ],
        diffAvailable: true,
        source: 'artifact' as const,
        warning: null,
      } as T;
    }
    case 'get_working_tree_status':
      return {
        clean: true,
        message: null,
      } as T;
    case 'preview_merge': {
      const args = _args as Record<string, unknown> | undefined;
      const agentKey = (args?.agentKey as string) ?? 'claude';
      return {
        agentKey,
        branch: `hydra/mock-run/agent/${agentKey}`,
        success: true,
        hasConflicts: false,
        stdout: 'Dry-run merge: clean merge (no conflicts)',
        stderr: '',
        reportPath: null,
      } as T;
    }
    case 'execute_merge': {
      const args = _args as Record<string, unknown> | undefined;
      const agentKey = (args?.agentKey as string) ?? 'claude';
      return {
        agentKey,
        branch: `hydra/mock-run/agent/${agentKey}`,
        success: true,
        message: `Merged '${agentKey}' branch 'hydra/mock-run/agent/${agentKey}'`,
        stdout: 'Merge made by the \'ort\' strategy.',
        stderr: null,
      } as T;
    }
    case 'start_interactive_session': {
      const args = _args as Record<string, unknown> | undefined;
      const request = (args?.request ?? {}) as Record<string, unknown>;
      const agentKey = (request.agentKey as string) ?? 'claude';
      const allowExp = (request.allowExperimental as boolean) ?? false;
      const unsafeModeReq = (request.unsafeMode as boolean) ?? false;
      const sourceRoot = typeof request.cwd === 'string' && request.cwd.trim().length > 0
        ? request.cwd.trim()
        : '/workspace';
      const repoRoot = sourceRoot;

      const adapter = MOCK_ADAPTERS.find((a) => a.key === agentKey);
      if (!adapter) {
        throw new Error(`[binary_missing] Adapter '${agentKey}' binary not found in PATH. Install it or configure the path in hydra.toml.`);
      }
      if (adapter.tier === 'experimental' && !allowExp) {
        throw new Error(
          `[experimental_blocked] Adapter '${agentKey}' is experimental. Enable 'Allow Experimental' and confirm the risk acknowledgment to use it in interactive mode.`,
        );
      }
      if (unsafeModeReq) {
        throw new Error(
          `[unsafe_blocked] Adapter '${agentKey}' does not support unsafe mode flags. Unsafe mode is only available for adapters with explicit sandbox bypass support.`,
        );
      }

      const sessionId = `mock-session-${Date.now()}`;
      const startedAt = new Date().toISOString();
      const hasActiveSameSource = Array.from(mockInteractiveSessions.values()).some(
        (session) => session.status === 'running' && session.sourceRoot === sourceRoot,
      );
      const worktreePath = hasActiveSameSource
        ? `${repoRoot}/.hydra/worktrees/interactive/${sessionId}/${agentKey}`
        : null;
      const effectiveCwd = worktreePath ?? sourceRoot;

      mockInteractiveSessions.set(sessionId, {
        sessionId,
        agentKey,
        status: 'running',
        startedAt,
        sourceRoot,
        repoRoot,
        effectiveCwd,
        worktreePath,
        eventCursor: 0,
        events: buildMockInteractiveEvents(sessionId, agentKey),
      });
      return {
        sessionId,
        agentKey,
        status: 'running',
        startedAt,
        sourceRoot,
        repoRoot,
        effectiveCwd,
        worktreePath,
      } as T;
    }
    case 'poll_interactive_events': {
      const args = _args as Record<string, unknown> | undefined;
      const sessionId = args?.sessionId as string;
      const cursor = (args?.cursor as number) ?? 0;
      const session = mockInteractiveSessions.get(sessionId);
      if (!session) {
        return {
          sessionId,
          events: [],
          nextCursor: cursor,
          done: true,
          status: 'unknown',
          error: 'Session not found',
        } as T;
      }
      const batch = session.events.slice(cursor, cursor + 3);
      const nextCursor = cursor + batch.length;
      const done = session.status !== 'running' && nextCursor >= session.events.length;
      return {
        sessionId,
        events: batch,
        nextCursor,
        done,
        status: session.status,
        error: null,
      } as T;
    }
    case 'write_interactive_input': {
      const args = _args as Record<string, unknown> | undefined;
      const sessionId = args?.sessionId as string;
      const input = args?.input as string;
      const session = mockInteractiveSessions.get(sessionId);
      if (!session || session.status !== 'running') {
        return {
          sessionId,
          success: false,
          error: session ? 'Session is not running' : 'Session not found',
        } as T;
      }
      session.events.push({
        sessionId,
        agentKey: session.agentKey,
        eventType: 'user_input',
        data: { input },
        timestamp: new Date().toISOString(),
      });
      session.events.push({
        sessionId,
        agentKey: session.agentKey,
        eventType: 'pty_output',
        data: { text: `Received: ${input}\n` },
        timestamp: new Date().toISOString(),
      });
      return { sessionId, success: true, error: null } as T;
    }
    case 'resize_interactive_terminal': {
      const args = _args as Record<string, unknown> | undefined;
      const sessionId = args?.sessionId as string;
      const cols = (args?.cols as number) ?? 80;
      const rows = (args?.rows as number) ?? 24;
      const session = mockInteractiveSessions.get(sessionId);
      if (!session || session.status !== 'running') {
        return {
          sessionId,
          success: false,
          cols,
          rows,
          error: session ? 'Session is not running' : 'Session not found',
        } as T;
      }
      return { sessionId, success: true, cols, rows, error: null } as T;
    }
    case 'stop_interactive_session': {
      const args = _args as Record<string, unknown> | undefined;
      const sessionId = args?.sessionId as string;
      const session = mockInteractiveSessions.get(sessionId);
      if (!session) {
        return { sessionId, status: 'unknown', wasRunning: false } as T;
      }
      const wasRunning = session.status === 'running';
      session.status = 'stopped';
      return { sessionId, status: 'stopped', wasRunning } as T;
    }
    case 'remove_interactive_session': {
      const args = _args as Record<string, unknown> | undefined;
      const sessionId = args?.sessionId as string;
      const session = mockInteractiveSessions.get(sessionId);
      if (!session) {
        throw new Error(`[not_found] session '${sessionId}' not found`);
      }
      if (session.status === 'running') {
        throw new Error('[validation_error] session is running; stop it before removing');
      }
      mockInteractiveSessions.delete(sessionId);
      return {
        sessionId,
        status: session.status,
        removed: true,
      } as T;
    }
    case 'list_interactive_sessions': {
      const summaries = Array.from(mockInteractiveSessions.values()).map((s) => ({
        sessionId: s.sessionId,
        agentKey: s.agentKey,
        status: s.status,
        startedAt: s.startedAt,
        eventCount: s.events.length,
        sourceRoot: s.sourceRoot,
        repoRoot: s.repoRoot,
        effectiveCwd: s.effectiveCwd,
        worktreePath: s.worktreePath,
      }));
      return summaries as T;
    }
    case 'get_interactive_transport_diagnostics': {
      const sessionId = (_args?.sessionId as string) ?? 'mock-session';
      return {
        sessionId,
        pushEmitErrorCount: 0,
        lastPushEmitError: null,
        lastPushEmitAt: null,
      } as T;
    }

    // File Explorer mock (P4.9.2)
    case 'list_directory': {
      const dirPath = (_args?.path as string) ?? '/workspace';
      return {
        path: dirPath,
        entries: [
          { name: 'src', path: `${dirPath}/src`, entryType: 'directory', size: null, modifiedAt: new Date().toISOString() },
          { name: 'Cargo.toml', path: `${dirPath}/Cargo.toml`, entryType: 'file', size: 512, modifiedAt: new Date().toISOString() },
          { name: 'README.md', path: `${dirPath}/README.md`, entryType: 'file', size: 1024, modifiedAt: new Date().toISOString() },
          { name: '.gitignore', path: `${dirPath}/.gitignore`, entryType: 'file', size: 64, modifiedAt: new Date().toISOString() },
        ],
        error: null,
      } as T;
    }

    case 'read_file_preview': {
      const filePath = (_args?.path as string) ?? '/workspace/README.md';
      const lower = filePath.toLowerCase();
      const isBinary = lower.endsWith('.png') || lower.endsWith('.ico') || lower.endsWith('.jpg');
      return {
        path: filePath,
        content: isBinary ? null : `# Mock Preview\n\nPreview for ${filePath}\n`,
        truncated: false,
        isBinary,
        size: isBinary ? 4096 : 256,
        error: null,
      } as T;
    }

    case 'start_file_watcher': {
      return {
        watcherId: `mock-watcher-${Date.now()}`,
        root: (_args?.root as string) ?? '/workspace',
      } as T;
    }

    case 'poll_file_watch_events': {
      return {
        watcherId: (_args?.watcherId as string) ?? 'mock-watcher',
        events: [],
        nextCursor: (_args?.cursor as number) ?? 0,
        active: true,
        error: null,
      } as T;
    }

    case 'stop_file_watcher': {
      return {
        watcherId: (_args?.watcherId as string) ?? 'mock-watcher',
        wasActive: true,
      } as T;
    }

    default:
      throw new Error(`Unknown command: ${cmd}`);
  }
}
