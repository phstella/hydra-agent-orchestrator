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
} from './types';

type InvokeFn = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

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
    default:
      throw new Error(`Unknown command: ${cmd}`);
  }
}
