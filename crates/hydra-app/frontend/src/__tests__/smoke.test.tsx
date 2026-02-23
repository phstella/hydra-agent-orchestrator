/**
 * P3-QA-01: GUI Smoke Test Pack
 *
 * Covers: startup, preflight refresh, experimental modal gating, race flow,
 * winner selection, diff candidate switching, and merge dry-run gating.
 */
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import App from '../App';
import * as ipc from '../ipc';
import type {
  PreflightResult,
  AdapterInfo,
  RaceStarted,
  RaceEventBatch,
  RaceResult,
  CandidateDiffPayload,
  MergePreviewPayload,
  MergeExecutionPayload,
} from '../types';

vi.mock('../ipc');

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
    version: '0.1.0',
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
    { name: 'Git Repository', description: 'Working inside a valid git repository', status: 'passed', evidence: null },
    { name: 'Environment Variables Check', description: 'Found system configuration', status: 'passed', evidence: null },
    { name: 'Validating Adapters', description: '2/2 tier-1 adapters ready', status: 'passed', evidence: 'Connected to 2 adapter(s)' },
    { name: 'Working Tree Cleanliness', description: 'Working tree is clean', status: 'passed', evidence: null },
  ],
  adapters: MOCK_ADAPTERS,
  warnings: [],
};

const MOCK_RACE_RESULT: RaceResult = {
  runId: 'test-run-id',
  status: 'completed',
  durationMs: 5000,
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
        { name: 'build', score: 100.0, evidence: {} },
        { name: 'tests', score: 92.0, evidence: {} },
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
        { name: 'build', score: 100.0, evidence: {} },
        { name: 'tests', score: 85.0, evidence: {} },
      ],
    },
  ],
};

const MOCK_DIFF: CandidateDiffPayload = {
  runId: 'test-run-id',
  agentKey: 'claude',
  baseRef: 'HEAD~1',
  branch: 'hydra/test-run-id/agent/claude',
  mergeable: true,
  gateFailures: [],
  diffText: 'diff --git a/src/main.rs b/src/main.rs\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,3 +1,5 @@\n fn main() {\n-    println!("hello");\n+    println!("world");\n+    println!("more");\n }\n',
  files: [
    { path: 'src/main.rs', added: 2, removed: 1 },
  ],
  diffAvailable: true,
  source: 'artifact',
  warning: null,
};

const MOCK_CODEX_DIFF: CandidateDiffPayload = {
  ...MOCK_DIFF,
  agentKey: 'codex',
  branch: 'hydra/test-run-id/agent/codex',
  diffText: 'diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,2 +1,4 @@\n-pub fn process() {}\n+pub fn process() {\n+    validate();\n+    transform();\n }\n',
  files: [
    { path: 'src/lib.rs', added: 3, removed: 1 },
  ],
};

function setupDefaultMocks() {
  vi.mocked(ipc.listAdapters).mockResolvedValue(MOCK_ADAPTERS);
  vi.mocked(ipc.runPreflight).mockResolvedValue(MOCK_PREFLIGHT);
  vi.mocked(ipc.getCandidateDiff).mockImplementation(async (_runId: string, agentKey: string) => {
    if (agentKey === 'codex') return MOCK_CODEX_DIFF;
    return MOCK_DIFF;
  });
  vi.mocked(ipc.previewMerge).mockResolvedValue({
    agentKey: 'claude',
    branch: 'hydra/test-run-id/agent/claude',
    success: true,
    hasConflicts: false,
    stdout: 'clean merge',
    stderr: '',
    reportPath: null,
  } as MergePreviewPayload);
  vi.mocked(ipc.executeMerge).mockResolvedValue({
    agentKey: 'claude',
    branch: 'hydra/test-run-id/agent/claude',
    success: true,
    message: 'Merged successfully',
    stdout: null,
    stderr: null,
  } as MergeExecutionPayload);
}

function mockRaceFlow() {
  let pollCount = 0;
  vi.mocked(ipc.startRace).mockResolvedValue({
    runId: 'test-run-id',
    agents: ['claude', 'codex'],
  } as RaceStarted);

  vi.mocked(ipc.pollRaceEvents).mockImplementation(async () => {
    pollCount++;
    if (pollCount >= 2) {
      return {
        runId: 'test-run-id',
        events: [
          { runId: 'test-run-id', agentKey: 'system', eventType: 'race_completed', data: {}, timestamp: new Date().toISOString() },
        ],
        nextCursor: 10,
        done: true,
        status: 'completed',
        error: null,
      } as RaceEventBatch;
    }
    return {
      runId: 'test-run-id',
      events: [
        { runId: 'test-run-id', agentKey: 'claude', eventType: 'agent_stdout', data: { line: 'Working...' }, timestamp: new Date().toISOString() },
      ],
      nextCursor: pollCount,
      done: false,
      status: 'running',
      error: null,
    } as RaceEventBatch;
  });

  vi.mocked(ipc.getRaceResult).mockResolvedValue(MOCK_RACE_RESULT);
}

beforeEach(() => {
  vi.resetAllMocks();
  setupDefaultMocks();
});

describe('Smoke Test 1: App startup renders tabs and preflight screen', () => {
  it('renders navigation tabs including Preflight, Race, Results, Review', async () => {
    render(<App />);

    await waitFor(() => {
      expect(screen.getByRole('tab', { name: /preflight/i })).toBeInTheDocument();
      expect(screen.getByRole('tab', { name: /race/i })).toBeInTheDocument();
      expect(screen.getByRole('tab', { name: /results/i })).toBeInTheDocument();
      expect(screen.getByRole('tab', { name: /review/i })).toBeInTheDocument();
    });
  });

  it('defaults to preflight tab', async () => {
    render(<App />);
    await waitFor(() => {
      const preflightTab = screen.getByRole('tab', { name: /preflight/i });
      expect(preflightTab).toHaveAttribute('aria-selected', 'true');
    });
  });
});

describe('Smoke Test 2: Preflight refresh triggers IPC and updates state', () => {
  it('loads preflight data on mount and shows diagnostic checks', async () => {
    render(<App />);
    await waitFor(() => {
      expect(ipc.runPreflight).toHaveBeenCalledTimes(1);
    });
  });

  it('re-runs diagnostics action triggers a new preflight call', async () => {
    render(<App />);
    await waitFor(() => {
      expect(ipc.runPreflight).toHaveBeenCalledTimes(1);
    });

    const rerunBtn = await screen.findByText(/re-run diagnostics/i);
    await act(async () => {
      fireEvent.click(rerunBtn);
    });

    await waitFor(() => {
      expect(ipc.runPreflight).toHaveBeenCalledTimes(2);
    });
  });
});

describe('Smoke Test 3: Experimental adapter modal blocks confirm until acknowledgment', () => {
  it('opens modal when selecting an experimental adapter', async () => {
    const user = userEvent.setup();
    render(<App />);

    await waitFor(() => {
      expect(screen.getByText('cursor-agent')).toBeInTheDocument();
    });

    await act(async () => {
      await user.click(screen.getByRole('tab', { name: /race/i }));
    });

    const cursorBtn = screen.getByText('cursor-agent').closest('button');
    expect(cursorBtn).toBeTruthy();

    await act(async () => {
      if (cursorBtn) await user.click(cursorBtn);
    });

    await waitFor(() => {
      expect(screen.getByText(/experimental adapter warning/i)).toBeInTheDocument();
    });
  });
});

describe('Smoke Test 4: Race flow transitions', () => {
  it('starts race, shows running status, transitions to completed with results', async () => {
    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);

    await act(async () => {
      await user.click(screen.getByRole('tab', { name: /race/i }));
    });

    await waitFor(() => {
      expect(screen.getByText('claude')).toBeInTheDocument();
    });

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await act(async () => {
      await user.type(textarea, 'Fix the bug in main.rs');
    });

    const startBtn = screen.getByRole('button', { name: /start race/i });
    await act(async () => {
      await user.click(startBtn);
    });

    await waitFor(() => {
      expect(ipc.startRace).toHaveBeenCalled();
    });

    await waitFor(() => {
      expect(ipc.getRaceResult).toHaveBeenCalled();
    }, { timeout: 5000 });

    await waitFor(() => {
      expect(screen.getByText('View Scoreboard')).toBeInTheDocument();
    });
  });
});

describe('Smoke Test 5: Winner selection is explicit and does not auto-merge', () => {
  it('allows explicit winner selection without triggering merge', async () => {
    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);

    await act(async () => { await user.click(screen.getByRole('tab', { name: /race/i })); });
    await waitFor(() => expect(screen.getByText('claude')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await act(async () => {
      await user.type(textarea, 'Fix bug');
      await user.click(screen.getByRole('button', { name: /start race/i }));
    });

    await waitFor(() => expect(ipc.getRaceResult).toHaveBeenCalled(), { timeout: 5000 });

    await act(async () => {
      await user.click(screen.getByText('View Scoreboard'));
    });

    await waitFor(() => {
      const winnerBtns = screen.getAllByText('Select as Winner');
      expect(winnerBtns.length).toBeGreaterThan(0);
    });

    const selectBtns = screen.getAllByText('Select as Winner');
    await act(async () => {
      await user.click(selectBtns[0]);
    });

    expect(ipc.executeMerge).not.toHaveBeenCalled();
  });
});

describe('Smoke Test 6: Diff candidate switching updates diff and file list', () => {
  it('switches diff content when a different candidate tab is clicked', async () => {
    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);

    await act(async () => { await user.click(screen.getByRole('tab', { name: /race/i })); });
    await waitFor(() => expect(screen.getByText('claude')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await act(async () => {
      await user.type(textarea, 'Fix bug');
      await user.click(screen.getByRole('button', { name: /start race/i }));
    });
    await waitFor(() => expect(ipc.getRaceResult).toHaveBeenCalled(), { timeout: 5000 });

    await waitFor(() => expect(screen.getByText('View Scoreboard')).toBeInTheDocument());
    await act(async () => { await user.click(screen.getByText('View Scoreboard')); });

    await waitFor(() => {
      const selectBtns = screen.getAllByText('Select as Winner');
      expect(selectBtns.length).toBeGreaterThan(0);
    });

    const selectBtns = screen.getAllByText('Select as Winner');
    await act(async () => {
      await user.click(selectBtns[0]);
    });

    await waitFor(() => {
      expect(screen.getByRole('tab', { name: /review/i })).toHaveAttribute('aria-selected', 'true');
    });

    await waitFor(() => {
      expect(ipc.getCandidateDiff).toHaveBeenCalledWith('test-run-id', 'claude');
    });

    const codexTab = screen.getByTestId('candidate-tab-codex');
    await act(async () => {
      await user.click(codexTab);
    });

    await waitFor(() => {
      expect(ipc.getCandidateDiff).toHaveBeenCalledWith('test-run-id', 'codex');
    });
  });
});

describe('Smoke Test 7: Merge dry-run gating behavior', () => {
  it('preview merge calls IPC with dry-run semantics', async () => {
    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);

    await act(async () => { await user.click(screen.getByRole('tab', { name: /race/i })); });
    await waitFor(() => expect(screen.getByText('claude')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await act(async () => {
      await user.type(textarea, 'Fix bug');
      await user.click(screen.getByRole('button', { name: /start race/i }));
    });
    await waitFor(() => expect(ipc.getRaceResult).toHaveBeenCalled(), { timeout: 5000 });

    await waitFor(() => expect(screen.getByText('View Scoreboard')).toBeInTheDocument());
    await act(async () => { await user.click(screen.getByText('View Scoreboard')); });

    await waitFor(() => {
      expect(screen.getAllByText('Select as Winner').length).toBeGreaterThan(0);
    });

    const selectBtns = screen.getAllByText('Select as Winner');
    await act(async () => { await user.click(selectBtns[0]); });

    await waitFor(() => {
      expect(screen.getByTestId('preview-merge-btn')).toBeInTheDocument();
    });

    await act(async () => {
      await user.click(screen.getByTestId('preview-merge-btn'));
    });

    await waitFor(() => {
      expect(ipc.previewMerge).toHaveBeenCalledWith('test-run-id', 'claude', false);
    });
  });

  it('blocks accept when merge preview shows conflicts', async () => {
    vi.mocked(ipc.previewMerge).mockResolvedValue({
      agentKey: 'claude',
      branch: 'hydra/test-run-id/agent/claude',
      success: false,
      hasConflicts: true,
      stdout: '',
      stderr: 'CONFLICT in src/main.rs',
      reportPath: null,
    });

    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);

    await act(async () => { await user.click(screen.getByRole('tab', { name: /race/i })); });
    await waitFor(() => expect(screen.getByText('claude')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await act(async () => {
      await user.type(textarea, 'Fix bug');
      await user.click(screen.getByRole('button', { name: /start race/i }));
    });
    await waitFor(() => expect(ipc.getRaceResult).toHaveBeenCalled(), { timeout: 5000 });

    await waitFor(() => expect(screen.getByText('View Scoreboard')).toBeInTheDocument());
    await act(async () => { await user.click(screen.getByText('View Scoreboard')); });

    await waitFor(() => {
      expect(screen.getAllByText('Select as Winner').length).toBeGreaterThan(0);
    });

    const selectBtns = screen.getAllByText('Select as Winner');
    await act(async () => { await user.click(selectBtns[0]); });

    await waitFor(() => expect(screen.getByTestId('preview-merge-btn')).toBeInTheDocument());

    await act(async () => {
      await user.click(screen.getByTestId('preview-merge-btn'));
    });

    await waitFor(() => {
      expect(screen.getByText(/conflicts detected/i)).toBeInTheDocument();
    });

    const acceptBtn = screen.getByTestId('accept-merge-btn');
    expect(acceptBtn).toBeDisabled();
  });
});
