/**
 * Tauri IPC bridge.
 *
 * When running outside Tauri (e.g. `npm run dev` standalone), falls back
 * to mock data so the UI can be developed without the Rust backend.
 */

import type {
  PreflightResult,
  AdapterInfo,
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
const MOCK_EVENT_STREAM = [
  { runId: 'mock-run', agentKey: 'system', eventType: 'race_process_started', data: {}, timestamp: new Date().toISOString() },
  { runId: 'mock-run', agentKey: 'claude', eventType: 'agent_started', data: {}, timestamp: new Date().toISOString() },
  { runId: 'mock-run', agentKey: 'claude', eventType: 'agent_stdout', data: { line: 'Analyzing repository...' }, timestamp: new Date().toISOString() },
];

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
      return { runId: 'mock-run', agents: ['claude', 'codex'] } as T;
    case 'get_race_result':
      return {
        runId: 'mock-run',
        status: 'completed',
        agents: [
          { agentKey: 'claude', status: 'completed', durationMs: 2500, score: 93.2, mergeable: true },
          { agentKey: 'codex', status: 'completed', durationMs: 2800, score: 91.1, mergeable: true },
        ],
      } as T;
    case 'poll_race_events': {
      const batch = MOCK_EVENT_STREAM.slice(mockCursor, mockCursor + 2);
      mockCursor += batch.length;
      return {
        runId: 'mock-run',
        events: batch,
        nextCursor: mockCursor,
        done: mockCursor >= MOCK_EVENT_STREAM.length,
        status: mockCursor >= MOCK_EVENT_STREAM.length ? 'completed' : 'running',
        error: null,
      } as T;
    }
    default:
      throw new Error(`Unknown command: ${cmd}`);
  }
}
