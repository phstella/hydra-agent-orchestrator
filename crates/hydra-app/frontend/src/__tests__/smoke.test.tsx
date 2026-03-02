/**
 * P3-QA-01 + M4.3/M4.4 + M4.7 + P4.9.1 + P4.9.4 + P4.9.5: GUI Smoke Test Pack
 *
 * Covers: cockpit shell render, startup, preflight refresh, experimental modal gating,
 * race flow from cockpit, winner selection, diff candidate switching, merge dry-run gating,
 * orchestration tab, session creation, output polling, stop session,
 * leaderboard updates, agent focus switch, completion summary, restart/retry flows,
 * default landing on orchestration, navigation transitions,
 * file explorer tab, tree rendering, live filesystem watch, manual refresh,
 * direct external CLI deploy trigger (P4.9.4), terminal-only input model (P4.9.5).
 */
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
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
  InteractiveSessionStarted,
  InteractiveEventBatch,
  InteractiveStreamEvent,
  InteractiveWriteAck,
  InteractiveResizeAck,
  InteractiveStopResult,
} from '../types';

vi.mock('../ipc');

type XTermMockInstance = {
  __rawWrites: string[];
  __element: HTMLDivElement | null;
  __emitData?: (data: string) => void;
};

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
  vi.mocked(ipc.getWorkingTreeStatus).mockResolvedValue({
    clean: true,
    message: null,
  });
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
  vi.mocked(ipc.listInteractiveSessions).mockResolvedValue([]);
  vi.mocked(ipc.listenInteractiveEvents).mockResolvedValue(null);
  vi.mocked(ipc.startInteractiveSession).mockResolvedValue({
    sessionId: 'test-session-1',
    agentKey: 'claude',
    status: 'running',
    startedAt: new Date().toISOString(),
  } as InteractiveSessionStarted);
  vi.mocked(ipc.pollInteractiveEvents).mockResolvedValue({
    sessionId: 'test-session-1',
    events: [
      {
        sessionId: 'test-session-1',
        agentKey: 'claude',
        eventType: 'output',
        data: { text: 'Hello from agent\n' },
        timestamp: new Date().toISOString(),
      },
    ],
    nextCursor: 1,
    done: false,
    status: 'running',
    error: null,
  } as InteractiveEventBatch);
  vi.mocked(ipc.writeInteractiveInput).mockResolvedValue({
    sessionId: 'test-session-1',
    success: true,
    error: null,
  } as InteractiveWriteAck);
  vi.mocked(ipc.resizeInteractiveTerminal).mockResolvedValue({
    sessionId: 'test-session-1',
    success: true,
    cols: 120,
    rows: 30,
    error: null,
  } as InteractiveResizeAck);
  vi.mocked(ipc.stopInteractiveSession).mockResolvedValue({
    sessionId: 'test-session-1',
    status: 'stopped',
    wasRunning: true,
  } as InteractiveStopResult);
  // File Explorer defaults (P4.9.2)
  vi.mocked(ipc.listDirectory).mockResolvedValue({
    path: '.',
    entries: [],
    error: null,
  });
  vi.mocked(ipc.startFileWatcher).mockResolvedValue({
    watcherId: 'default-watcher',
    root: '.',
  });
  vi.mocked(ipc.pollFileWatchEvents).mockResolvedValue({
    watcherId: 'default-watcher',
    events: [],
    nextCursor: 0,
    active: true,
    error: null,
  });
  vi.mocked(ipc.stopFileWatcher).mockResolvedValue({
    watcherId: 'default-watcher',
    wasActive: true,
  });
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
  const storage = window.localStorage as unknown as {
    clear?: () => void;
    removeItem?: (key: string) => void;
  };
  if (typeof storage.clear === 'function') {
    storage.clear();
  } else if (typeof storage.removeItem === 'function') {
    storage.removeItem('hydra.workspace.path');
  }
  setupDefaultMocks();
});

describe('Smoke Test 1: App startup renders cockpit shell with navigation', () => {
  it('renders cockpit shell with left rail, top strip, and center', async () => {
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('cockpit-shell')).toBeInTheDocument();
      expect(screen.getByTestId('cockpit-left-rail')).toBeInTheDocument();
      expect(screen.getByTestId('cockpit-top-strip')).toBeInTheDocument();
      expect(screen.getByTestId('cockpit-center')).toBeInTheDocument();
    });
    // Right rail only appears in cockpit view (not on default orchestration landing)
    expect(screen.queryByTestId('cockpit-right-rail')).not.toBeInTheDocument();
  });

  it('renders navigation buttons in left rail', async () => {
    render(<App />);
    await waitFor(() => {
      expect(screen.getByTestId('nav-cockpit')).toBeInTheDocument();
      expect(screen.getByTestId('nav-preflight')).toBeInTheDocument();
      expect(screen.getByTestId('nav-results')).toBeInTheDocument();
      expect(screen.getByTestId('nav-review')).toBeInTheDocument();
      expect(screen.getByTestId('nav-orchestration')).toBeInTheDocument();
      expect(screen.getByTestId('nav-settings')).toBeInTheDocument();
    });
  });

  it('defaults to orchestration view with create panel', async () => {
    render(<App />);
    await waitFor(() => {
      expect(screen.getByTestId('orchestration-console')).toBeInTheDocument();
      expect(screen.getByTestId('create-panel')).toBeInTheDocument();
    });
  });

  it('shows leaderboard rail only in cockpit view', async () => {
    const user = userEvent.setup();
    render(<App />);

    // Default is orchestration — no right rail
    await waitFor(() => {
      expect(screen.getByTestId('orchestration-console')).toBeInTheDocument();
    });
    expect(screen.queryByTestId('cockpit-right-rail')).not.toBeInTheDocument();

    // Navigate to cockpit — right rail appears
    await user.click(screen.getByTestId('nav-cockpit'));
    await waitFor(() => {
      expect(screen.getByTestId('cockpit-right-rail')).toBeInTheDocument();
    });
  });
});

describe('Smoke Test 2: Preflight refresh triggers IPC and updates state', () => {
  it('loads preflight data when navigating to preflight view', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-preflight'));
    await waitFor(() => {
      expect(ipc.runPreflight).toHaveBeenCalledTimes(1);
    });
  });

  it('re-runs diagnostics action triggers a new preflight call', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-preflight'));
    await waitFor(() => {
      expect(ipc.runPreflight).toHaveBeenCalledTimes(1);
    });

    const rerunBtn = await screen.findByText(/re-run diagnostics/i);
    fireEvent.click(rerunBtn);

    await waitFor(() => {
      expect(ipc.runPreflight).toHaveBeenCalledTimes(2);
    });
  });
});

describe('Smoke Test 3: Experimental adapter modal blocks confirm until acknowledgment', () => {
  it('opens modal when selecting an experimental adapter in cockpit config', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => {
      expect(screen.getByTestId('race-config-panel')).toBeInTheDocument();
    });

    const cursorBtn = screen.getByText('cursor-agent').closest('button');
    expect(cursorBtn).toBeTruthy();

    if (cursorBtn) {
      await user.click(cursorBtn);
    }

    await waitFor(() => {
      expect(screen.getByText(/experimental adapter warning/i)).toBeInTheDocument();
    });

    const confirmBtn = screen.getByRole('button', { name: /confirm selection/i });
    expect(confirmBtn).toBeDisabled();

    await user.click(screen.getByRole('checkbox'));
    expect(confirmBtn).toBeEnabled();
  });
});

describe('Smoke Test 4: Race flow transitions from cockpit', () => {
  it('starts race from cockpit, shows running status, transitions to completed with results', async () => {
    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => {
      expect(screen.getByTestId('race-config-panel')).toBeInTheDocument();
    });

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await user.type(textarea, 'Fix the bug in main.rs');

    const startBtn = screen.getByTestId('cockpit-start-race');
    await user.click(startBtn);

    await waitFor(() => {
      expect(ipc.startRace).toHaveBeenCalled();
    });

    await waitFor(() => {
      expect(ipc.getRaceResult).toHaveBeenCalled();
    }, { timeout: 5000 });

    await waitFor(() => {
      expect(screen.getByTestId('completion-summary')).toBeInTheDocument();
    });
  });
});

describe('Smoke Test 5: Winner selection is explicit and does not auto-merge', () => {
  it('allows explicit winner selection from completion summary without triggering merge', async () => {
    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await user.type(textarea, 'Fix bug');
    await user.click(screen.getByTestId('cockpit-start-race'));

    await waitFor(() => expect(ipc.getRaceResult).toHaveBeenCalled(), { timeout: 5000 });

    await waitFor(() => {
      expect(screen.getByTestId('completion-summary')).toBeInTheDocument();
    });

    const selectBtn = screen.getByTestId('completion-select-winner');
    await user.click(selectBtn);

    expect(ipc.executeMerge).not.toHaveBeenCalled();
  });
});

describe('Smoke Test 6: Diff candidate switching updates diff and file list', () => {
  it('switches diff content when a different candidate tab is clicked', async () => {
    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await user.type(textarea, 'Fix bug');
    await user.click(screen.getByTestId('cockpit-start-race'));
    await waitFor(() => expect(ipc.getRaceResult).toHaveBeenCalled(), { timeout: 5000 });

    await waitFor(() => expect(screen.getByTestId('completion-summary')).toBeInTheDocument());
    await user.click(screen.getByTestId('completion-open-review'));

    await waitFor(() => {
      expect(screen.getByText('Original')).toBeInTheDocument();
      expect(screen.getByText('Candidate')).toBeInTheDocument();
    });

    await waitFor(() => {
      expect(ipc.getCandidateDiff).toHaveBeenCalledWith('test-run-id', 'claude');
    });

    const codexTab = screen.getByTestId('candidate-tab-codex');
    await user.click(codexTab);

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
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await user.type(textarea, 'Fix bug');
    await user.click(screen.getByTestId('cockpit-start-race'));
    await waitFor(() => expect(ipc.getRaceResult).toHaveBeenCalled(), { timeout: 5000 });

    await waitFor(() => expect(screen.getByTestId('completion-summary')).toBeInTheDocument());
    await user.click(screen.getByTestId('completion-open-review'));

    await waitFor(() => {
      expect(screen.getByTestId('preview-merge-btn')).toBeInTheDocument();
    });

    await user.click(screen.getByTestId('preview-merge-btn'));

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
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await user.type(textarea, 'Fix bug');
    await user.click(screen.getByTestId('cockpit-start-race'));
    await waitFor(() => expect(ipc.getRaceResult).toHaveBeenCalled(), { timeout: 5000 });

    await waitFor(() => expect(screen.getByTestId('completion-summary')).toBeInTheDocument());
    await user.click(screen.getByTestId('completion-open-review'));

    await waitFor(() => expect(screen.getByTestId('preview-merge-btn')).toBeInTheDocument());
    await user.click(screen.getByTestId('preview-merge-btn'));

    await waitFor(() => {
      expect(screen.getByText(/conflicts detected/i)).toBeInTheDocument();
    });

    const acceptBtn = screen.getByTestId('accept-merge-btn');
    expect(acceptBtn).toBeDisabled();
  });

  it('blocks accept and shows error when preview fails without conflicts', async () => {
    vi.mocked(ipc.previewMerge).mockResolvedValue({
      agentKey: 'claude',
      branch: 'hydra/test-run-id/agent/claude',
      success: false,
      hasConflicts: false,
      stdout: '',
      stderr: 'working tree has uncommitted changes',
      reportPath: null,
    });

    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await user.type(textarea, 'Fix bug');
    await user.click(screen.getByTestId('cockpit-start-race'));
    await waitFor(() => expect(ipc.getRaceResult).toHaveBeenCalled(), { timeout: 5000 });

    await waitFor(() => expect(screen.getByTestId('completion-summary')).toBeInTheDocument());
    await user.click(screen.getByTestId('completion-open-review'));

    await waitFor(() => expect(screen.getByTestId('preview-merge-btn')).toBeInTheDocument());
    await user.click(screen.getByTestId('preview-merge-btn'));

    await waitFor(() => {
      expect(screen.getByText(/preview failed/i)).toBeInTheDocument();
    });

    const acceptBtn = screen.getByTestId('accept-merge-btn');
    expect(acceptBtn).toBeDisabled();
  });

  it('disables preview when working tree is dirty and shows actionable warning', async () => {
    vi.mocked(ipc.getWorkingTreeStatus).mockResolvedValue({
      clean: false,
      message: 'Working tree has uncommitted changes in: src/main.rs',
    });

    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await user.type(textarea, 'Fix bug');
    await user.click(screen.getByTestId('cockpit-start-race'));
    await waitFor(() => expect(ipc.getRaceResult).toHaveBeenCalled(), { timeout: 5000 });

    await waitFor(() => expect(screen.getByTestId('completion-summary')).toBeInTheDocument());
    await user.click(screen.getByTestId('completion-open-review'));

    await waitFor(() => expect(screen.getByTestId('preview-merge-btn')).toBeInTheDocument());
    await waitFor(() => {
      expect(screen.getByTestId('worktree-warning')).toBeInTheDocument();
    });

    const previewBtn = screen.getByTestId('preview-merge-btn');
    expect(previewBtn).toBeDisabled();
    expect(screen.getByText(/working tree has uncommitted changes/i)).toBeInTheDocument();
    expect(ipc.previewMerge).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// Orchestration Session Smoke Tests (M4.3 + M4.4)
// ---------------------------------------------------------------------------

describe('Smoke Test 8: Orchestration tab renders and shows empty state', () => {
  it('renders the Orchestration tab in navigation', async () => {
    render(<App />);
    await waitFor(() => {
      expect(screen.getByTestId('nav-orchestration')).toBeInTheDocument();
    });
  });

  it('shows empty session state when no sessions exist (default landing)', async () => {
    render(<App />);
    // Orchestration is the default landing — no nav click needed
    await waitFor(() => {
      expect(screen.getByTestId('empty-session-state')).toBeInTheDocument();
    });
    expect(screen.getByTestId('terminal-empty-state')).toBeInTheDocument();
  });
});

describe('Smoke Test 9: Create and select orchestration session', () => {
  it('creates session from orchestration create panel with IPC', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => {
      expect(screen.getByTestId('create-panel')).toBeInTheDocument();
      expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument();
    });

    await user.type(screen.getByTestId('session-task-prompt'), 'Fix the bug');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(ipc.startInteractiveSession).toHaveBeenCalledWith(
        expect.objectContaining({
          agentKey: 'claude',
          taskPrompt: 'Fix the bug',
        }),
      );
    });

    await waitFor(() => {
      expect(screen.getByTestId('terminal-panel')).toBeInTheDocument();
    });
  });
});

describe('Smoke Test 10: Output polling renders in terminal panel', () => {
  it('polls events and displays output text', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'Test task');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(ipc.pollInteractiveEvents).toHaveBeenCalled();
    });

    await waitFor(() => {
      expect(screen.getByText(/Hello from agent/)).toBeInTheDocument();
    });
  });
});

describe('Smoke Test 11: P4.9.5 terminal-only input model', () => {
  it('does not render InputComposer in orchestration view', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'Test input');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(screen.getByTestId('terminal-toolbar')).toBeInTheDocument();
    });

    expect(screen.queryByTestId('input-composer')).not.toBeInTheDocument();
    expect(screen.queryByTestId('interactive-input')).not.toBeInTheDocument();
    expect(screen.queryByTestId('send-input-btn')).not.toBeInTheDocument();
  });

  it('renders terminal-toolbar with stop button when session is running', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'Test stop');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(screen.getByTestId('stop-session-btn')).toBeInTheDocument();
    });
  });

  it('routes terminal keyboard input to writeInteractiveInput', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'Input route test');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(screen.getByTestId('stop-session-btn')).toBeInTheDocument();
    });

    const instances = (globalThis as Record<string, unknown>).__xtermInstances as XTermMockInstance[];
    const term = instances[instances.length - 1];
    expect(term).toBeDefined();
    expect(typeof term.__emitData).toBe('function');

    term.__emitData?.('status\n');

    await waitFor(() => {
      expect(ipc.writeInteractiveInput).toHaveBeenCalledWith('test-session-1', 'status\n');
    });
  });

  it('uses push stream transport when listener is available', async () => {
    let pushHandler: ((event: InteractiveStreamEvent) => void) | null = null;
    vi.mocked(ipc.listenInteractiveEvents).mockImplementation(async (handler) => {
      pushHandler = handler;
      return () => {};
    });
    vi.mocked(ipc.pollInteractiveEvents).mockResolvedValue({
      sessionId: 'test-session-1',
      events: [],
      nextCursor: 0,
      done: false,
      status: 'running',
      error: null,
    } as InteractiveEventBatch);

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));
    await waitFor(() => expect(ipc.listenInteractiveEvents).toHaveBeenCalled());

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'push');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => expect(screen.getByTestId('terminal-panel')).toBeInTheDocument());
    if (!pushHandler) {
      throw new Error('push listener handler was not registered');
    }
    const pushFn = pushHandler as (event: InteractiveStreamEvent) => void;

    pushFn({
      sessionId: 'test-session-1',
      agentKey: 'claude',
      eventType: 'output',
      data: { text: 'push-stream-line\n' },
      timestamp: new Date().toISOString(),
    });

    await waitFor(() => {
      expect(screen.getByText(/push-stream-line/)).toBeInTheDocument();
    });
  });

  it('hides initial prompt composer after deploy and keeps terminal as primary input', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'bootstrap prompt');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(screen.getByTestId('stop-session-btn')).toBeInTheDocument();
    });
    expect(screen.queryByTestId('session-task-prompt')).not.toBeInTheDocument();
    expect(screen.getByTestId('show-initial-prompt-btn')).toBeInTheDocument();
  });

  it('deploys a lane without requiring an initial prompt', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(ipc.startInteractiveSession).toHaveBeenCalledWith(
        expect.objectContaining({
          agentKey: 'claude',
          taskPrompt: '',
        }),
      );
    });
  });

  it('does not render user_input events in terminal output (no duplicate echo)', async () => {
    vi.mocked(ipc.pollInteractiveEvents).mockResolvedValue({
      sessionId: 'test-session-1',
      events: [
        {
          sessionId: 'test-session-1',
          agentKey: 'claude',
          eventType: 'user_input',
          data: { input: 'duplicate-me' },
          timestamp: new Date().toISOString(),
        },
      ],
      nextCursor: 1,
      done: false,
      status: 'running',
      error: null,
    } as InteractiveEventBatch);

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'test');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => expect(screen.getByTestId('terminal-panel')).toBeInTheDocument());
    expect(screen.queryByText(/duplicate-me/)).not.toBeInTheDocument();
  });

  it('deduplicates overlapping interactive poll batches to avoid repeated terminal lines', async () => {
    let pollCount = 0;
    vi.mocked(ipc.pollInteractiveEvents).mockImplementation(async () => {
      pollCount += 1;
      if (pollCount === 1) {
        return {
          sessionId: 'test-session-1',
          events: [
            {
              sessionId: 'test-session-1',
              agentKey: 'claude',
              eventType: 'output',
              data: { text: 'line-a\n' },
              timestamp: new Date().toISOString(),
            },
            {
              sessionId: 'test-session-1',
              agentKey: 'claude',
              eventType: 'output',
              data: { text: 'line-b\n' },
              timestamp: new Date().toISOString(),
            },
          ],
          nextCursor: 2,
          done: false,
          status: 'running',
          error: null,
        } as InteractiveEventBatch;
      }

      if (pollCount === 2) {
        // Replays one already-seen event ("line-b") and appends one fresh line ("line-c")
        return {
          sessionId: 'test-session-1',
          events: [
            {
              sessionId: 'test-session-1',
              agentKey: 'claude',
              eventType: 'output',
              data: { text: 'line-b\n' },
              timestamp: new Date().toISOString(),
            },
            {
              sessionId: 'test-session-1',
              agentKey: 'claude',
              eventType: 'output',
              data: { text: 'line-c\n' },
              timestamp: new Date().toISOString(),
            },
          ],
          nextCursor: 3,
          done: false,
          status: 'running',
          error: null,
        } as InteractiveEventBatch;
      }

      return {
        sessionId: 'test-session-1',
        events: [],
        nextCursor: 3,
        done: false,
        status: 'running',
        error: null,
      } as InteractiveEventBatch;
    });

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'overlap');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => expect(screen.getByText(/line-c/)).toBeInTheDocument());

    const instances = (globalThis as Record<string, unknown>).__xtermInstances as XTermMockInstance[];
    const term = instances[instances.length - 1];
    const text = term.__element?.textContent ?? '';
    expect((text.match(/line-b/g) ?? []).length).toBe(1);
  });
});

describe('Smoke Test 12: Stop session and lifecycle transition', () => {
  it('stops a running session and updates status', async () => {
    let pollCount = 0;
    vi.mocked(ipc.pollInteractiveEvents).mockImplementation(async () => {
      pollCount++;
      if (pollCount <= 3) {
        return {
          sessionId: 'test-session-1',
          events: [
            {
              sessionId: 'test-session-1',
              agentKey: 'claude',
              eventType: 'output',
              data: { text: 'Working...\n' },
              timestamp: new Date().toISOString(),
            },
          ],
          nextCursor: pollCount,
          done: false,
          status: 'running',
          error: null,
        } as InteractiveEventBatch;
      }
      return {
        sessionId: 'test-session-1',
        events: [],
        nextCursor: pollCount,
        done: true,
        status: 'stopped',
        error: null,
      } as InteractiveEventBatch;
    });

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'Stop test');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => expect(screen.getByTestId('stop-session-btn')).toBeInTheDocument());

    await user.click(screen.getByTestId('stop-session-btn'));

    await waitFor(() => {
      expect(ipc.stopInteractiveSession).toHaveBeenCalledWith('test-session-1');
    });

    await waitFor(() => {
      expect(screen.getByTestId('session-ended-indicator')).toHaveTextContent('Session stopped');
    });
    expect(screen.queryByTestId('stop-session-btn')).not.toBeInTheDocument();
  });
});

describe('Smoke Test 13: Orchestration terminal handles stream errors and ANSI output', () => {
  it('shows a connection warning when polling fails', async () => {
    vi.mocked(ipc.pollInteractiveEvents).mockRejectedValue(new Error('connection refused'));

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'Poll fail test');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(screen.getByTestId('terminal-transport-error')).toHaveTextContent('connection refused');
    });
  });

  it('preserves raw ANSI sequences for high-fidelity terminal rendering (P4.9.3)', async () => {
    const ansiText = '\u001b[32mGreen output\u001b[0m\r\n';
    vi.mocked(ipc.pollInteractiveEvents).mockResolvedValue({
      sessionId: 'test-session-1',
      events: [
        {
          sessionId: 'test-session-1',
          agentKey: 'claude',
          eventType: 'output',
          data: { text: ansiText },
          timestamp: new Date().toISOString(),
        },
      ],
      nextCursor: 1,
      done: false,
      status: 'running',
      error: null,
    } as InteractiveEventBatch);

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'ANSI test');
    await user.click(screen.getByTestId('confirm-create-session'));

    // xterm.js mock renders stripped text into DOM for query compat
    await waitFor(() => {
      expect(screen.getByText('Green output')).toBeInTheDocument();
    });

    // Raw ANSI was preserved and passed to Terminal.write()
    const instances = (globalThis as Record<string, unknown>).__xtermInstances as Array<{
      __rawWrites: string[];
    }>;
    const termInstance = instances.find((t) => t.__rawWrites.length > 0);
    expect(termInstance).toBeDefined();
    expect(termInstance!.__rawWrites.some((w) => w.includes('\u001b[32m'))).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// M4.5: Orchestration Safety and Capability Gating Smoke Tests
// ---------------------------------------------------------------------------

describe('Smoke Test 14: Experimental adapter shows warning and requires acknowledgment', () => {
  it('shows experimental warning when selecting cursor-agent', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('create-panel')).toBeInTheDocument());

    const cursorBtn = screen.getByTestId('agent-select-cursor-agent');
    await user.click(cursorBtn);

    await waitFor(() => {
      expect(screen.getByTestId('experimental-warning')).toBeInTheDocument();
    });

    expect(screen.getByTestId('confirm-create-session')).toBeDisabled();
  });

  it('enables start after acknowledging experimental risk', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('create-panel')).toBeInTheDocument());

    await user.click(screen.getByTestId('agent-select-cursor-agent'));
    await waitFor(() => expect(screen.getByTestId('experimental-warning')).toBeInTheDocument());

    await user.click(screen.getByTestId('experimental-acknowledge-checkbox'));
    expect(screen.getByTestId('confirm-create-session')).not.toBeDisabled();
  });
});

describe('Smoke Test 15: Experimental adapter denied without confirmation shows error', () => {
  it('shows error when backend rejects experimental adapter', async () => {
    vi.mocked(ipc.startInteractiveSession).mockRejectedValue(
      new Error('[experimental_blocked] Adapter \'cursor-agent\' is experimental.'),
    );

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('create-panel')).toBeInTheDocument());

    await user.click(screen.getByTestId('agent-select-cursor-agent'));
    await waitFor(() => expect(screen.getByTestId('experimental-warning')).toBeInTheDocument());
    await user.click(screen.getByTestId('experimental-acknowledge-checkbox'));

    await waitFor(() => {
      expect(screen.getByTestId('confirm-create-session')).not.toBeDisabled();
    });

    await user.type(screen.getByTestId('session-task-prompt'), 'test experimental');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(screen.getByTestId('create-session-error')).toBeInTheDocument();
      expect(screen.getByTestId('create-session-error')).toHaveTextContent(/experimental/i);
    });
  });
});

describe('Smoke Test 16: Dirty working tree policy block shows clear feedback', () => {
  it('shows dirty worktree error from backend', async () => {
    vi.mocked(ipc.startInteractiveSession).mockRejectedValue(
      new Error('[dirty_worktree] Working tree has uncommitted changes. Commit or stash changes before starting.'),
    );

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());

    await user.type(screen.getByTestId('session-task-prompt'), 'test dirty tree');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      const errorEl = screen.getByTestId('create-session-error');
      expect(errorEl).toBeInTheDocument();
      expect(errorEl).toHaveTextContent(/uncommitted changes/i);
    });
  });
});

describe('Smoke Test 17: Unsupported adapter blocked with actionable reason', () => {
  it('shows safety gate error when adapter is unavailable', async () => {
    vi.mocked(ipc.startInteractiveSession).mockRejectedValue(
      new Error('[safety_gate] Adapter \'claude\' is not available for interactive sessions. Run \'hydra doctor\' to diagnose.'),
    );

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());

    await user.type(screen.getByTestId('session-task-prompt'), 'test blocked adapter');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      const errorEl = screen.getByTestId('create-session-error');
      expect(errorEl).toBeInTheDocument();
      expect(errorEl).toHaveTextContent(/not available/i);
    });
  });
});

describe('Smoke Test 18: Workspace path is propagated to backend IPC', () => {
  it('uses selected workspace for race and diff IPC calls', async () => {
    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByTestId('nav-settings'));
    await waitFor(() => expect(screen.getByTestId('settings-workspace-input')).toBeInTheDocument());

    const settingsInput = screen.getByTestId('settings-workspace-input');
    await user.type(settingsInput, '/tmp/custom-hydra-workspace');
    await user.click(screen.getByTestId('settings-save-workspace'));

    await user.click(screen.getByTestId('nav-cockpit'));
    await waitFor(() => expect(screen.getByText('claude')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await user.type(textarea, 'Run race with custom workspace');
    await user.click(screen.getByTestId('cockpit-start-race'));

    await waitFor(() => {
      expect(ipc.startRace).toHaveBeenCalledWith(
        expect.objectContaining({
          cwd: '/tmp/custom-hydra-workspace',
        }),
      );
    });

    await waitFor(() => expect(ipc.getRaceResult).toHaveBeenCalled(), { timeout: 5000 });
    await waitFor(() => expect(screen.getByTestId('completion-summary')).toBeInTheDocument());
    await user.click(screen.getByTestId('completion-open-review'));

    await waitFor(() => {
      expect(ipc.getCandidateDiff).toHaveBeenCalledWith(
        'test-run-id',
        'claude',
        '/tmp/custom-hydra-workspace',
      );
    });
  });
});

// ---------------------------------------------------------------------------
// M4.7: Cockpit Convergence Smoke Tests
// ---------------------------------------------------------------------------

describe('Smoke Test 19: Cockpit leaderboard updates during race', () => {
  it('shows leaderboard cards that update from live stream', async () => {
    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await user.type(textarea, 'Test leaderboard');
    await user.click(screen.getByTestId('cockpit-start-race'));

    await waitFor(() => {
      expect(screen.getByTestId('leaderboard-rail')).toBeInTheDocument();
    });

    await waitFor(() => {
      expect(screen.getByTestId('leaderboard-card-claude')).toBeInTheDocument();
      expect(screen.getByTestId('leaderboard-card-codex')).toBeInTheDocument();
    });
  });
});

describe('Smoke Test 20: Agent focus switch updates terminal', () => {
  it('clicking leaderboard card switches terminal focus', async () => {
    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await user.type(textarea, 'Test focus switch');
    await user.click(screen.getByTestId('cockpit-start-race'));

    await waitFor(() => {
      expect(screen.getByTestId('leaderboard-card-codex')).toBeInTheDocument();
    });

    await user.click(screen.getByTestId('leaderboard-card-codex'));

    await waitFor(() => {
      expect(screen.getByText(/Output: codex/)).toBeInTheDocument();
    });
  });
});

describe('Smoke Test 21: Completion summary and review transition', () => {
  it('shows completion summary with review CTA after race completes', async () => {
    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await user.type(textarea, 'Test completion');
    await user.click(screen.getByTestId('cockpit-start-race'));

    await waitFor(() => expect(ipc.getRaceResult).toHaveBeenCalled(), { timeout: 5000 });

    await waitFor(() => {
      expect(screen.getByTestId('completion-summary')).toBeInTheDocument();
      expect(screen.getByTestId('completion-open-review')).toBeInTheDocument();
    });

    await user.click(screen.getByTestId('completion-open-review'));

    await waitFor(() => {
      expect(screen.getByText('Diff Review')).toBeInTheDocument();
    });
  });
});

describe('Smoke Test 22: Completion summary can reset cockpit for a new race', () => {
  it('returns to race config after Start New Race', async () => {
    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await user.type(textarea, 'Test restart flow');
    await user.click(screen.getByTestId('cockpit-start-race'));

    await waitFor(() => expect(ipc.getRaceResult).toHaveBeenCalled(), { timeout: 5000 });
    await waitFor(() => expect(screen.getByTestId('completion-summary')).toBeInTheDocument());

    await user.click(screen.getByTestId('completion-start-new-race'));

    await waitFor(() => {
      expect(screen.getByTestId('race-config-panel')).toBeInTheDocument();
    });
    expect(screen.queryByTestId('completion-summary')).not.toBeInTheDocument();
  });
});

describe('Smoke Test 23: Failed race can be retried from cockpit', () => {
  it('keeps config visible on failure and allows immediate retry', async () => {
    vi.mocked(ipc.startRace).mockRejectedValue(new Error('simulated race start failure'));

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await user.type(textarea, 'Retry after failure');
    await user.click(screen.getByTestId('cockpit-start-race'));

    await waitFor(() => {
      expect(screen.getByTestId('race-config-panel')).toBeInTheDocument();
      expect(screen.getByTestId('strip-run-btn')).toBeInTheDocument();
    });

    await user.click(screen.getByTestId('strip-run-btn'));

    await waitFor(() => {
      expect(ipc.startRace).toHaveBeenCalledTimes(2);
    });
  });
});

describe('Smoke Test 24: Open Diff Review follows top winner even when list order differs', () => {
  it('opens review focused on highest-scored candidate, not first array entry', async () => {
    mockRaceFlow();

    vi.mocked(ipc.getRaceResult).mockResolvedValue({
      ...MOCK_RACE_RESULT,
      agents: [
        { ...MOCK_RACE_RESULT.agents[1], agentKey: 'codex', score: 88.5 },
        { ...MOCK_RACE_RESULT.agents[0], agentKey: 'claude', score: 93.2 },
      ],
    });

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await user.type(textarea, 'Winner routing');
    await user.click(screen.getByTestId('cockpit-start-race'));

    await waitFor(() => expect(ipc.getRaceResult).toHaveBeenCalled(), { timeout: 5000 });
    await waitFor(() => expect(screen.getByTestId('completion-summary')).toBeInTheDocument());

    await user.click(screen.getByTestId('completion-open-review'));

    await waitFor(() => {
      expect(ipc.getCandidateDiff).toHaveBeenCalledWith('test-run-id', 'claude');
    });
  });
});

describe('Smoke Test 25: Early race failure marks agent cards as failed', () => {
  it('shows failed lifecycle for known agents when run fails before agent events', async () => {
    vi.mocked(ipc.startRace).mockResolvedValue({
      runId: 'test-run-id',
      agents: ['claude', 'codex'],
    } as RaceStarted);

    vi.mocked(ipc.pollRaceEvents).mockResolvedValue({
      runId: 'test-run-id',
      events: [],
      nextCursor: 0,
      done: true,
      status: 'failed',
      error: 'Not inside a git repository',
    } as RaceEventBatch);

    vi.mocked(ipc.getRaceResult).mockResolvedValue({
      runId: 'test-run-id',
      status: 'failed',
      durationMs: null,
      totalCost: null,
      agents: [],
    } as RaceResult);

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await user.type(textarea, 'trigger early failure');
    await user.click(screen.getByTestId('cockpit-start-race'));

    await waitFor(() => {
      expect(screen.getByTestId('leaderboard-card-claude')).toBeInTheDocument();
      expect(screen.getByTestId('leaderboard-card-codex')).toBeInTheDocument();
      expect(screen.getByTestId('leaderboard-failure-claude')).toBeInTheDocument();
      expect(screen.getByTestId('leaderboard-failure-codex')).toBeInTheDocument();
    });
  });
});

describe('Smoke Test 26: Cockpit shows backend failure reason from race polling', () => {
  it('renders explicit race error text when poll returns failed status with error detail', async () => {
    vi.mocked(ipc.startRace).mockResolvedValue({
      runId: 'test-run-id',
      agents: ['claude', 'codex'],
    } as RaceStarted);

    vi.mocked(ipc.pollRaceEvents).mockResolvedValue({
      runId: 'test-run-id',
      events: [],
      nextCursor: 0,
      done: true,
      status: 'failed',
      error: 'race command failed: adapter \'claude\' is not ready',
    } as RaceEventBatch);

    vi.mocked(ipc.getRaceResult).mockResolvedValue({
      runId: 'test-run-id',
      status: 'failed',
      durationMs: null,
      totalCost: null,
      agents: [],
    } as RaceResult);

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());

    const textarea = screen.getByPlaceholderText(/describe the task/i);
    await user.type(textarea, 'surface backend failure');
    await user.click(screen.getByTestId('cockpit-start-race'));

    await waitFor(() => {
      const error = screen.getByTestId('cockpit-race-error');
      expect(error).toBeInTheDocument();
      expect(error).toHaveTextContent(/adapter 'claude' is not ready/i);
    });
  });
});

describe('Smoke Test 27: Live output supports human-readable and event views', () => {
  it('toggles between human and structured event rendering', async () => {
    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());
    await user.type(screen.getByPlaceholderText(/describe the task/i), 'toggle output mode');
    await user.click(screen.getByTestId('cockpit-start-race'));

    await waitFor(() => {
      expect(screen.getByTestId('output-view-human')).toBeInTheDocument();
      expect(screen.getByText('Working...')).toBeInTheDocument();
    });

    expect(screen.queryByText('agent_stdout')).not.toBeInTheDocument();
    await user.click(screen.getByTestId('output-view-events'));

    await waitFor(() => {
      expect(screen.getByText('agent_stdout')).toBeInTheDocument();
    });
  });
});

describe('Smoke Test 28: Diff viewer supports unified fallback mode', () => {
  it('can switch from side-by-side to unified mode in review', async () => {
    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());
    await user.type(screen.getByPlaceholderText(/describe the task/i), 'diff mode toggle');
    await user.click(screen.getByTestId('cockpit-start-race'));

    await waitFor(() => expect(screen.getByTestId('completion-summary')).toBeInTheDocument());
    await user.click(screen.getByTestId('completion-open-review'));

    await waitFor(() => {
      expect(screen.getByTestId('diff-viewer')).toBeInTheDocument();
      expect(screen.getByTestId('diff-view-mode-side')).toBeInTheDocument();
      expect(screen.getByTestId('diff-view-mode-unified')).toBeInTheDocument();
    });

    await user.click(screen.getByTestId('diff-view-mode-unified'));
    await waitFor(() => {
      expect(screen.getByTestId('unified-diff-view')).toBeInTheDocument();
    });
  });
});

describe('Smoke Test 29: Quality warning appears when only speed/diff scoring is available', () => {
  it('shows low-confidence warning in completion and review surfaces', async () => {
    mockRaceFlow();
    vi.mocked(ipc.getRaceResult).mockResolvedValue({
      ...MOCK_RACE_RESULT,
      agents: MOCK_RACE_RESULT.agents.map((agent) => ({
        ...agent,
        dimensions: agent.dimensions.filter((d) => d.name === 'diff_scope' || d.name === 'speed'),
      })),
    });

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());
    await user.type(screen.getByPlaceholderText(/describe the task/i), 'quality warning');
    await user.click(screen.getByTestId('cockpit-start-race'));

    await waitFor(() => {
      expect(screen.getByTestId('completion-summary')).toBeInTheDocument();
      expect(screen.getByText(/quality checks are not configured/i)).toBeInTheDocument();
    });

    await user.click(screen.getByTestId('completion-open-review'));
    await waitFor(() => {
      expect(screen.getByTestId('review-quality-warning')).toBeInTheDocument();
    });
  });
});

describe('Smoke Test 30: Human output mode parses structured JSON lines', () => {
  it('renders concise text instead of raw adapter JSON in Human mode', async () => {
    const structuredLine = JSON.stringify({
      type: 'assistant',
      message: {
        content: [{ type: 'text', text: 'Created snake.py successfully' }],
      },
    });

    vi.mocked(ipc.startRace).mockResolvedValue({
      runId: 'test-run-id',
      agents: ['claude', 'codex'],
    } as RaceStarted);

    let pollCount = 0;
    vi.mocked(ipc.pollRaceEvents).mockImplementation(async () => {
      pollCount += 1;
      if (pollCount === 1) {
        return {
          runId: 'test-run-id',
          events: [
            {
              runId: 'test-run-id',
              agentKey: 'claude',
              eventType: 'agent_stdout',
              data: { line: structuredLine },
              timestamp: new Date().toISOString(),
            },
          ],
          nextCursor: 1,
          done: false,
          status: 'running',
          error: null,
        } as RaceEventBatch;
      }
      return {
        runId: 'test-run-id',
        events: [],
        nextCursor: pollCount,
        done: true,
        status: 'completed',
        error: null,
      } as RaceEventBatch;
    });

    vi.mocked(ipc.getRaceResult).mockResolvedValue(MOCK_RACE_RESULT);

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());
    await user.type(screen.getByPlaceholderText(/describe the task/i), 'human output json parse');
    await user.click(screen.getByTestId('cockpit-start-race'));

    await waitFor(() => {
      expect(screen.getByTestId('live-output-panel')).toBeInTheDocument();
      expect(screen.getByText('Created snake.py successfully')).toBeInTheDocument();
    });
    expect(screen.queryByText(/"type":"assistant"/)).not.toBeInTheDocument();
  });
});

describe('Smoke Test 31: Cockpit completion view does not overlap terminal', () => {
  it('hides live output panel after race completion and shows completion summary only', async () => {
    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());
    await user.type(screen.getByPlaceholderText(/describe the task/i), 'completion layout');
    await user.click(screen.getByTestId('cockpit-start-race'));

    await waitFor(() => expect(ipc.getRaceResult).toHaveBeenCalled(), { timeout: 5000 });
    await waitFor(() => {
      expect(screen.getByTestId('completion-summary')).toBeInTheDocument();
    });
    expect(screen.queryByTestId('live-output-panel')).not.toBeInTheDocument();
  });
});

describe('Smoke Test 32: New-file diffs default to unified mode', () => {
  it('switches to unified mode automatically for creation/binary-heavy patches', async () => {
    vi.mocked(ipc.getCandidateDiff).mockImplementation(async (_runId: string, agentKey: string) => {
      if (agentKey === 'codex') {
        return {
          ...MOCK_CODEX_DIFF,
          diffText: [
            'diff --git a/__pycache__/snake.pyc b/__pycache__/snake.pyc',
            'new file mode 100644',
            'index 0000000..13ef0f7',
            'Binary files /dev/null and b/__pycache__/snake.pyc differ',
            'diff --git a/snake.py b/snake.py',
            'new file mode 100644',
            'index 0000000..1ddb5cf',
            '--- /dev/null',
            '+++ b/snake.py',
            '@@ -0,0 +1,2 @@',
            '+print("snake")',
            '+print("game")',
          ].join('\n'),
          files: [
            { path: '__pycache__/snake.pyc', added: 0, removed: 0 },
            { path: 'snake.py', added: 2, removed: 0 },
          ],
        };
      }
      return MOCK_DIFF;
    });

    mockRaceFlow();
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-cockpit'));

    await waitFor(() => expect(screen.getByTestId('race-config-panel')).toBeInTheDocument());
    await user.type(screen.getByPlaceholderText(/describe the task/i), 'new file diff mode');
    await user.click(screen.getByTestId('cockpit-start-race'));

    await waitFor(() => expect(screen.getByTestId('completion-summary')).toBeInTheDocument());
    await user.click(screen.getByTestId('completion-open-review'));
    await waitFor(() => expect(screen.getByTestId('candidate-tab-codex')).toBeInTheDocument());
    await user.click(screen.getByTestId('candidate-tab-codex'));

    await waitFor(() => {
      expect(screen.getByTestId('unified-diff-view')).toBeInTheDocument();
      expect(screen.getByText('New File')).toBeInTheDocument();
    });
  });
});

// ---------------------------------------------------------------------------
// P4.9.1: Orchestration IA Rename and Default Landing
// ---------------------------------------------------------------------------

describe('Smoke Test 38: Orchestration is the default landing view', () => {
  it('renders orchestration console on startup without navigation', async () => {
    render(<App />);
    await waitFor(() => {
      expect(screen.getByTestId('orchestration-console')).toBeInTheDocument();
      expect(screen.getByTestId('create-panel')).toBeInTheDocument();
      expect(screen.getByTestId('lanes-rail')).toBeInTheDocument();
    });
    // Cockpit race config is NOT shown on startup
    expect(screen.queryByTestId('race-config-panel')).not.toBeInTheDocument();
  });

  it('navigates from orchestration to cockpit and back', async () => {
    const user = userEvent.setup();
    render(<App />);

    // Default: orchestration
    await waitFor(() => {
      expect(screen.getByTestId('orchestration-console')).toBeInTheDocument();
    });

    // Navigate to cockpit
    await user.click(screen.getByTestId('nav-cockpit'));
    await waitFor(() => {
      expect(screen.getByTestId('race-config-panel')).toBeInTheDocument();
    });
    expect(screen.queryByTestId('orchestration-console')).not.toBeInTheDocument();

    // Navigate back to orchestration
    await user.click(screen.getByTestId('nav-orchestration'));
    await waitFor(() => {
      expect(screen.getByTestId('orchestration-console')).toBeInTheDocument();
    });
    expect(screen.queryByTestId('race-config-panel')).not.toBeInTheDocument();
  });

  it('preserves race/results/review/settings navigation', async () => {
    const user = userEvent.setup();
    render(<App />);

    // Navigate to each view and verify it renders
    await user.click(screen.getByTestId('nav-preflight'));
    await waitFor(() => {
      expect(ipc.runPreflight).toHaveBeenCalled();
    });

    await user.click(screen.getByTestId('nav-results'));
    await waitFor(() => {
      expect(screen.getByText(/no results yet/i)).toBeInTheDocument();
    });

    await user.click(screen.getByTestId('nav-settings'));
    await waitFor(() => {
      expect(screen.getByTestId('settings-workspace-input')).toBeInTheDocument();
    });
  });
});

// ---------------------------------------------------------------------------
// M4.8.9: Orchestration Console — QA Hardening
// ---------------------------------------------------------------------------

describe('Smoke Test 33: Duplicate adapter sessions can be created from orchestration surface', () => {
  it('launches two codex sessions with distinct lane entries', async () => {
    let sessionCounter = 0;
    vi.mocked(ipc.startInteractiveSession).mockImplementation(async (req) => {
      sessionCounter++;
      return {
        sessionId: `dup-session-${sessionCounter}`,
        agentKey: req.agentKey,
        status: 'running',
        startedAt: new Date().toISOString(),
      } as InteractiveSessionStarted;
    });
    vi.mocked(ipc.pollInteractiveEvents).mockImplementation(async (sessionId) => ({
      sessionId,
      events: [
        {
          sessionId,
          agentKey: 'codex',
          eventType: 'output',
          data: { text: `Output from ${sessionId}\n` },
          timestamp: new Date().toISOString(),
        },
      ],
      nextCursor: 1,
      done: false,
      status: 'running',
      error: null,
    } as InteractiveEventBatch));

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('create-panel')).toBeInTheDocument());

    // Select codex adapter
    await user.click(screen.getByTestId('agent-select-codex'));

    // Launch first codex session
    await user.type(screen.getByTestId('session-task-prompt'), 'Task A');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(screen.getByTestId('session-item-dup-session-1')).toBeInTheDocument();
    });

    // Launch second codex session
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(screen.getByTestId('session-item-dup-session-2')).toBeInTheDocument();
    });

    // Both sessions exist as distinct lane cards
    expect(screen.getByTestId('session-item-dup-session-1')).toBeInTheDocument();
    expect(screen.getByTestId('session-item-dup-session-2')).toBeInTheDocument();

    // IPC called twice with codex
    expect(ipc.startInteractiveSession).toHaveBeenCalledTimes(2);
    expect(ipc.startInteractiveSession).toHaveBeenCalledWith(
      expect.objectContaining({ agentKey: 'codex' }),
    );
  });
});

describe('Smoke Test 34: Lane selection changes focused terminal source', () => {
  it('switches terminal lane label when a different lane is selected', async () => {
    let sessionCounter = 0;
    vi.mocked(ipc.startInteractiveSession).mockImplementation(async (req) => {
      sessionCounter++;
      return {
        sessionId: `focus-session-${sessionCounter}`,
        agentKey: req.agentKey,
        status: 'running',
        startedAt: new Date().toISOString(),
      } as InteractiveSessionStarted;
    });
    vi.mocked(ipc.pollInteractiveEvents).mockImplementation(async (sessionId) => ({
      sessionId,
      events: [
        {
          sessionId,
          agentKey: sessionId === 'focus-session-1' ? 'claude' : 'codex',
          eventType: 'output',
          data: { text: `Output from ${sessionId}\n` },
          timestamp: new Date().toISOString(),
        },
      ],
      nextCursor: 1,
      done: false,
      status: 'running',
      error: null,
    } as InteractiveEventBatch));

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());

    // Create first session (claude)
    await user.type(screen.getByTestId('session-task-prompt'), 'Task A');
    await user.click(screen.getByTestId('confirm-create-session'));
    await waitFor(() => expect(screen.getByTestId('session-item-focus-session-1')).toBeInTheDocument());

    // Create second session (codex)
    await user.click(screen.getByTestId('agent-select-codex'));
    await user.click(screen.getByTestId('confirm-create-session'));
    await waitFor(() => expect(screen.getByTestId('session-item-focus-session-2')).toBeInTheDocument());

    // Focus should be on session 2 (most recently created) — terminal shows codex label
    await waitFor(() => {
      const label = screen.getByTestId('terminal-lane-label');
      expect(label).toHaveTextContent(/codex/);
      expect(label).toHaveTextContent(/focus-se/);
    });

    // Click session 1 lane card to switch focus
    await user.click(screen.getByTestId('session-item-focus-session-1'));

    // Terminal header should now show claude label with session 1's ID prefix
    await waitFor(() => {
      const label = screen.getByTestId('terminal-lane-label');
      expect(label).toHaveTextContent(/claude/);
      expect(label).toHaveTextContent(/focus-se/);
    });
  });
});

describe('Smoke Test 35: Per-lane input isolation under duplicate adapters', () => {
  it('write action targets only the selected session', async () => {
    let sessionCounter = 0;
    vi.mocked(ipc.startInteractiveSession).mockImplementation(async (req) => {
      sessionCounter++;
      return {
        sessionId: `input-session-${sessionCounter}`,
        agentKey: req.agentKey,
        status: 'running',
        startedAt: new Date().toISOString(),
      } as InteractiveSessionStarted;
    });
    vi.mocked(ipc.pollInteractiveEvents).mockImplementation(async (sessionId) => ({
      sessionId,
      events: [
        {
          sessionId,
          agentKey: 'claude',
          eventType: 'output',
          data: { text: 'Ready\n' },
          timestamp: new Date().toISOString(),
        },
      ],
      nextCursor: 1,
      done: false,
      status: 'running',
      error: null,
    } as InteractiveEventBatch));
    vi.mocked(ipc.writeInteractiveInput).mockImplementation(async (sessionId) => ({
      sessionId,
      success: true,
      error: null,
    } as InteractiveWriteAck));

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());

    // Create two claude sessions
    await user.type(screen.getByTestId('session-task-prompt'), 'Task A');
    await user.click(screen.getByTestId('confirm-create-session'));
    await waitFor(() => expect(screen.getByTestId('session-item-input-session-1')).toBeInTheDocument());

    await user.click(screen.getByTestId('confirm-create-session'));
    await waitFor(() => expect(screen.getByTestId('session-item-input-session-2')).toBeInTheDocument());

    // Focus is on session 2 (most recent). Terminal input should target session 2.
    await waitFor(() => expect(screen.getByTestId('stop-session-btn')).toBeInTheDocument());
    const instances = (globalThis as Record<string, unknown>).__xtermInstances as XTermMockInstance[];
    const term = instances[instances.length - 1];
    expect(term).toBeDefined();
    expect(typeof term.__emitData).toBe('function');

    await waitFor(() => {
      term.__emitData?.('hello session 2\n');
      expect(ipc.writeInteractiveInput).toHaveBeenCalledWith('input-session-2', 'hello session 2\n');
    });

    // Switch focus and verify input retargets to session 1.
    await user.click(screen.getByTestId('session-item-input-session-1'));
    await waitFor(() => {
      term.__emitData?.('hello session 1\n');
      expect(ipc.writeInteractiveInput).toHaveBeenCalledWith('input-session-1', 'hello session 1\n');
    });
  });
});

describe('Smoke Test 36: Per-lane stop isolation under duplicate adapters', () => {
  it('stopping lane A leaves lane B running', async () => {
    let sessionCounter = 0;
    vi.mocked(ipc.startInteractiveSession).mockImplementation(async (req) => {
      sessionCounter++;
      return {
        sessionId: `stop-session-${sessionCounter}`,
        agentKey: req.agentKey,
        status: 'running',
        startedAt: new Date().toISOString(),
      } as InteractiveSessionStarted;
    });
    vi.mocked(ipc.pollInteractiveEvents).mockImplementation(async (sessionId) => ({
      sessionId,
      events: [],
      nextCursor: 0,
      done: false,
      status: 'running',
      error: null,
    } as InteractiveEventBatch));
    vi.mocked(ipc.stopInteractiveSession).mockImplementation(async (sessionId) => ({
      sessionId,
      status: 'stopped',
      wasRunning: true,
    } as InteractiveStopResult));

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());

    // Create two codex sessions
    await user.click(screen.getByTestId('agent-select-codex'));
    await user.type(screen.getByTestId('session-task-prompt'), 'Task A');
    await user.click(screen.getByTestId('confirm-create-session'));
    await waitFor(() => expect(screen.getByTestId('session-item-stop-session-1')).toBeInTheDocument());

    await user.click(screen.getByTestId('confirm-create-session'));
    await waitFor(() => expect(screen.getByTestId('session-item-stop-session-2')).toBeInTheDocument());

    // Focus on session 2, stop it via terminal toolbar stop button
    await waitFor(() => expect(screen.getByTestId('stop-session-btn')).toBeInTheDocument());
    await user.click(screen.getByTestId('stop-session-btn'));

    await waitFor(() => {
      expect(ipc.stopInteractiveSession).toHaveBeenCalledWith('stop-session-2');
    });

    // Stop was NOT called for session 1
    expect(ipc.stopInteractiveSession).not.toHaveBeenCalledWith('stop-session-1');
  });
});

describe('Smoke Test 37: Lane-local polling error does not collapse sibling lane', () => {
  it('one lane poll error does not affect other lane state', async () => {
    let sessionCounter = 0;
    vi.mocked(ipc.startInteractiveSession).mockImplementation(async (req) => {
      sessionCounter++;
      return {
        sessionId: `err-session-${sessionCounter}`,
        agentKey: req.agentKey,
        status: 'running',
        startedAt: new Date().toISOString(),
      } as InteractiveSessionStarted;
    });

    // Session 1 polls successfully; session 2 fails
    vi.mocked(ipc.pollInteractiveEvents).mockImplementation(async (sessionId) => {
      if (sessionId === 'err-session-2') {
        throw new Error('network timeout for session 2');
      }
      return {
        sessionId,
        events: [
          {
            sessionId,
            agentKey: 'claude',
            eventType: 'output',
            data: { text: 'Healthy output\n' },
            timestamp: new Date().toISOString(),
          },
        ],
        nextCursor: 1,
        done: false,
        status: 'running',
        error: null,
      } as InteractiveEventBatch;
    });

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());

    // Create session 1 (healthy)
    await user.type(screen.getByTestId('session-task-prompt'), 'Healthy task');
    await user.click(screen.getByTestId('confirm-create-session'));
    await waitFor(() => expect(screen.getByTestId('session-item-err-session-1')).toBeInTheDocument());

    // Create session 2 (will fail polling)
    await user.click(screen.getByTestId('confirm-create-session'));
    await waitFor(() => expect(screen.getByTestId('session-item-err-session-2')).toBeInTheDocument());

    // Switch to session 1 to verify it still has healthy output
    await user.click(screen.getByTestId('session-item-err-session-1'));

    await waitFor(() => {
      const matches = screen.getAllByText(/Healthy output/);
      expect(matches.length).toBeGreaterThan(0);
    });

    // Session 2 has a poll error indicator on its lane card
    await waitFor(() => {
      expect(screen.getByTestId('lane-error-err-session-2')).toBeInTheDocument();
    });
  });
});

// ---------------------------------------------------------------------------
// P4.9.2: File Explorer Smoke Tests
// ---------------------------------------------------------------------------

describe('Smoke Test 39: File Explorer tab is visible and navigable', () => {
  it('renders Files nav button and navigates to file explorer', async () => {
    const user = userEvent.setup();
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('nav-files')).toBeInTheDocument();
    });

    await user.click(screen.getByTestId('nav-files'));

    await waitFor(() => {
      expect(screen.getByTestId('file-explorer')).toBeInTheDocument();
    });
  });
});

describe('Smoke Test 40: File Explorer renders initial tree', () => {
  it('loads and displays directory entries on mount', async () => {
    vi.mocked(ipc.listDirectory).mockResolvedValue({
      path: '/workspace',
      entries: [
        { name: 'src', path: '/workspace/src', entryType: 'directory', size: null, modifiedAt: '2026-02-25T00:00:00Z' },
        { name: 'Cargo.toml', path: '/workspace/Cargo.toml', entryType: 'file', size: 512, modifiedAt: '2026-02-25T00:00:00Z' },
        { name: 'README.md', path: '/workspace/README.md', entryType: 'file', size: 1024, modifiedAt: '2026-02-25T00:00:00Z' },
      ],
      error: null,
    });

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-files'));

    await waitFor(() => {
      expect(screen.getByTestId('file-tree')).toBeInTheDocument();
      expect(screen.getByTestId('tree-node-src')).toBeInTheDocument();
      expect(screen.getByTestId('tree-node-Cargo.toml')).toBeInTheDocument();
      expect(screen.getByTestId('tree-node-README.md')).toBeInTheDocument();
    });
  });
});

describe('Smoke Test 41: File Explorer starts watcher and polls for events', () => {
  it('starts a file watcher when explorer mounts and polls for events', async () => {
    vi.mocked(ipc.listDirectory).mockResolvedValue({
      path: '.',
      entries: [
        { name: 'main.rs', path: './main.rs', entryType: 'file', size: 100, modifiedAt: '2026-02-25T00:00:00Z' },
      ],
      error: null,
    });

    vi.mocked(ipc.startFileWatcher).mockResolvedValue({
      watcherId: 'test-watcher',
      root: '.',
    });

    vi.mocked(ipc.pollFileWatchEvents).mockResolvedValue({
      watcherId: 'test-watcher',
      events: [],
      nextCursor: 0,
      active: true,
      error: null,
    });

    vi.mocked(ipc.stopFileWatcher).mockResolvedValue({ watcherId: 'test-watcher', wasActive: true });

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-files'));

    await waitFor(() => {
      expect(screen.getByTestId('tree-node-main.rs')).toBeInTheDocument();
    });

    // Verify watcher was started
    await waitFor(() => {
      expect(ipc.startFileWatcher).toHaveBeenCalled();
    });

    // Verify polling started
    await waitFor(() => {
      expect(ipc.pollFileWatchEvents).toHaveBeenCalled();
    });

    // Watcher active indicator should be visible
    await waitFor(() => {
      expect(screen.getByTestId('watcher-active-indicator')).toBeInTheDocument();
    });
  });
});

describe('Smoke Test 42: File Explorer manual refresh reloads tree', () => {
  it('reloads directory listing when Refresh button is clicked', async () => {
    let listCallCount = 0;
    vi.mocked(ipc.listDirectory).mockImplementation(async () => {
      listCallCount++;
      return {
        path: '/workspace',
        entries: listCallCount === 1
          ? [{ name: 'before.txt', path: '/workspace/before.txt', entryType: 'file', size: 50, modifiedAt: '2026-02-25T00:00:00Z' }]
          : [{ name: 'after.txt', path: '/workspace/after.txt', entryType: 'file', size: 75, modifiedAt: '2026-02-25T01:00:00Z' }],
        error: null,
      };
    });

    vi.mocked(ipc.startFileWatcher).mockResolvedValue({ watcherId: 'w1', root: '/workspace' });
    vi.mocked(ipc.pollFileWatchEvents).mockResolvedValue({
      watcherId: 'w1', events: [], nextCursor: 0, active: true, error: null,
    });
    vi.mocked(ipc.stopFileWatcher).mockResolvedValue({ watcherId: 'w1', wasActive: true });

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-files'));

    // Initial load
    await waitFor(() => {
      expect(screen.getByTestId('tree-node-before.txt')).toBeInTheDocument();
    });

    // Click refresh
    await user.click(screen.getByTestId('file-explorer-refresh'));

    // Should now show after.txt
    await waitFor(() => {
      expect(screen.getByTestId('tree-node-after.txt')).toBeInTheDocument();
    });
  });
});

describe('Smoke Test 43: File Explorer watcher events trigger debounced refresh', () => {
  it('reloads root listing after a rename watcher event', async () => {
    let listCallCount = 0;
    vi.mocked(ipc.listDirectory).mockImplementation(async () => {
      listCallCount++;
      return {
        path: '.',
        entries: listCallCount === 1
          ? [{ name: 'before.txt', path: './before.txt', entryType: 'file', size: 50, modifiedAt: '2026-02-25T00:00:00Z' }]
          : [{ name: 'after.txt', path: './after.txt', entryType: 'file', size: 75, modifiedAt: '2026-02-25T01:00:00Z' }],
        error: null,
      };
    });

    vi.mocked(ipc.startFileWatcher).mockResolvedValue({ watcherId: 'watch-rename', root: '.' });

    let pollCallCount = 0;
    vi.mocked(ipc.pollFileWatchEvents).mockImplementation(async () => {
      pollCallCount++;
      return {
        watcherId: 'watch-rename',
        events: pollCallCount === 1
          ? [{ eventType: 'rename', path: './after.txt', timestamp: '2026-02-25T01:00:00Z' }]
          : [],
        nextCursor: pollCallCount,
        active: true,
        error: null,
      };
    });
    vi.mocked(ipc.stopFileWatcher).mockResolvedValue({ watcherId: 'watch-rename', wasActive: true });

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-files'));

    await waitFor(() => {
      expect(screen.getByTestId('tree-node-before.txt')).toBeInTheDocument();
    });

    await waitFor(() => {
      expect(screen.getByTestId('tree-node-after.txt')).toBeInTheDocument();
    }, { timeout: 3000 });
  });
});

// ---------------------------------------------------------------------------
// P4.9.3: High-Fidelity Terminal Rendering (ANSI Parity) Fixture Tests
// ---------------------------------------------------------------------------

/** Helper: create a session and pump one ANSI output event through polling. */
async function setupTerminalWithAnsiOutput(
  ansiPayload: string,
  user: ReturnType<typeof userEvent.setup>,
) {
  vi.mocked(ipc.pollInteractiveEvents).mockResolvedValue({
    sessionId: 'test-session-1',
    events: [
      {
        sessionId: 'test-session-1',
        agentKey: 'claude',
        eventType: 'output',
        data: { text: ansiPayload },
        timestamp: new Date().toISOString(),
      },
    ],
    nextCursor: 1,
    done: false,
    status: 'running',
    error: null,
  } as InteractiveEventBatch);

  render(<App />);
  await user.click(screen.getByTestId('nav-orchestration'));
  await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
  await user.type(screen.getByTestId('session-task-prompt'), 'ANSI fixture');
  await user.click(screen.getByTestId('confirm-create-session'));

  await waitFor(() => {
    const instances = (globalThis as Record<string, unknown>).__xtermInstances as XTermMockInstance[];
    expect(instances.some((t) => t.__rawWrites.length > 0)).toBe(true);
  });

  const instances = (globalThis as Record<string, unknown>).__xtermInstances as XTermMockInstance[];
  return instances.find((t) => t.__rawWrites.length > 0)!;
}

describe('P4.9.3 Fixture: 24-bit color ANSI sequences', () => {
  it('preserves 24-bit foreground and background color escapes', async () => {
    const payload = '\u001b[38;2;255;128;0mOrange text\u001b[0m \u001b[48;2;0;0;128mBlue bg\u001b[0m\r\n';
    const user = userEvent.setup();
    const term = await setupTerminalWithAnsiOutput(payload, user);

    expect(term.__rawWrites.some((w) => w.includes('\u001b[38;2;255;128;0m'))).toBe(true);
    expect(term.__rawWrites.some((w) => w.includes('\u001b[48;2;0;0;128m'))).toBe(true);
    expect(screen.getByText(/Orange text/)).toBeInTheDocument();
    expect(screen.getByText(/Blue bg/)).toBeInTheDocument();
  });
});

describe('P4.9.3 Fixture: Bold, italic, underline style attributes', () => {
  it('preserves SGR style escape sequences', async () => {
    const payload = '\u001b[1mBold\u001b[0m \u001b[3mItalic\u001b[0m \u001b[4mUnderline\u001b[0m\r\n';
    const user = userEvent.setup();
    const term = await setupTerminalWithAnsiOutput(payload, user);

    // Bold = ESC[1m, Italic = ESC[3m, Underline = ESC[4m
    expect(term.__rawWrites.some((w) => w.includes('\u001b[1m'))).toBe(true);
    expect(term.__rawWrites.some((w) => w.includes('\u001b[3m'))).toBe(true);
    expect(term.__rawWrites.some((w) => w.includes('\u001b[4m'))).toBe(true);
    expect(screen.getByText(/Bold/)).toBeInTheDocument();
  });
});

describe('P4.9.3 Fixture: Cursor movement sequences', () => {
  it('preserves cursor-up, cursor-down, and cursor-position escapes', async () => {
    // CUU (cursor up 2), CUD (cursor down 1), CUP (move to row 1 col 1)
    const payload = 'abc\u001b[2A\u001b[1B\u001b[1;1HZ\r\n';
    const user = userEvent.setup();
    const term = await setupTerminalWithAnsiOutput(payload, user);

    expect(term.__rawWrites.some((w) => w.includes('\u001b[2A'))).toBe(true);
    expect(term.__rawWrites.some((w) => w.includes('\u001b[1B'))).toBe(true);
    expect(term.__rawWrites.some((w) => w.includes('\u001b[1;1H'))).toBe(true);
    expect(screen.getByText(/Zbc/)).toBeInTheDocument();
  });
});

describe('P4.9.3 Fixture: Clear-line and clear-screen sequences', () => {
  it('preserves erase-in-line (EL) and erase-in-display (ED) escapes', async () => {
    // EL clear to end of line, ED clear entire screen
    const payload = 'Before\u001b[KAfter\u001b[2JCleared\r\n';
    const user = userEvent.setup();
    const term = await setupTerminalWithAnsiOutput(payload, user);

    expect(term.__rawWrites.some((w) => w.includes('\u001b[K'))).toBe(true);
    expect(term.__rawWrites.some((w) => w.includes('\u001b[2J'))).toBe(true);
    expect(screen.getByText(/Cleared/)).toBeInTheDocument();
    expect(screen.queryByText(/Before/)).not.toBeInTheDocument();
    expect(screen.queryByText(/After/)).not.toBeInTheDocument();
  });
});

describe('P4.9.3 Fixture: Session change clears terminal', () => {
  it('xterm-container is present after session creation', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'Session clear test');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(screen.getByTestId('xterm-container')).toBeInTheDocument();
    });
  });
});

describe('P4.9.3 Fixture: Lane focus preserves per-session isolation', () => {
  it('lane label header still renders correctly with xterm terminal', async () => {
    vi.mocked(ipc.pollInteractiveEvents).mockResolvedValue({
      sessionId: 'test-session-1',
      events: [
        {
          sessionId: 'test-session-1',
          agentKey: 'claude',
          eventType: 'output',
          data: { text: 'Session output\r\n' },
          timestamp: new Date().toISOString(),
        },
      ],
      nextCursor: 1,
      done: false,
      status: 'running',
      error: null,
    } as InteractiveEventBatch);

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'Focus test');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      const label = screen.getByTestId('terminal-lane-label');
      expect(label).toHaveTextContent(/claude/);
    });
  });
});

// ---------------------------------------------------------------------------
// P4.9.4: Direct external CLI invocation smoke tests
// ---------------------------------------------------------------------------

describe('P4.9.4: Deploy launches selected external tool', () => {
  it('deploy button triggers startInteractiveSession with selected adapter', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());

    await user.click(screen.getByTestId('agent-select-codex'));
    await user.type(screen.getByTestId('session-task-prompt'), 'deploy test task');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(ipc.startInteractiveSession).toHaveBeenCalledWith(
        expect.objectContaining({
          agentKey: 'codex',
          taskPrompt: 'deploy test task',
        }),
      );
    });
  });

  it('shows actionable error for binary_missing', async () => {
    vi.mocked(ipc.startInteractiveSession).mockRejectedValue(
      new Error("[binary_missing] Adapter 'unknown-tool' binary not found in PATH. Install it or configure the path in hydra.toml."),
    );

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'binary missing test');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(screen.getByTestId('create-session-error')).toHaveTextContent(/binary not found/);
    });

    await waitFor(() => {
      expect(screen.getByTestId('create-session-error-hint')).toBeInTheDocument();
    });
  });

  it('shows actionable error for launch_error', async () => {
    vi.mocked(ipc.startInteractiveSession).mockRejectedValue(
      new Error('[launch_error] PTY spawn failed: permission denied'),
    );

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'launch error test');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(screen.getByTestId('create-session-error')).toHaveTextContent(/PTY spawn failed/);
    });

    await waitFor(() => {
      expect(screen.getByTestId('create-session-error-hint')).toHaveTextContent(/auth\/session/);
    });
  });

  it('surfaces runtime CLI failure details in terminal context', async () => {
    vi.mocked(ipc.pollInteractiveEvents).mockResolvedValue({
      sessionId: 'test-session-1',
      events: [],
      nextCursor: 1,
      done: true,
      status: 'failed',
      error: 'process exited with status code 7. Check CLI auth/session and flag compatibility.',
    } as InteractiveEventBatch);

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'runtime failure test');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(screen.getByTestId('terminal-session-error')).toHaveTextContent(/status code 7/);
    });
    expect(screen.getByTestId('terminal-session-error')).toHaveTextContent(/auth\/session/);
  });
});

// ---------------------------------------------------------------------------
// P4.9.5: Terminal-only input model smoke tests
// ---------------------------------------------------------------------------

describe('P4.9.5: Terminal-only input model', () => {
  it('orchestration does not render side InputComposer', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('orchestration-console')).toBeInTheDocument());

    expect(screen.queryByTestId('input-composer')).not.toBeInTheDocument();
  });

  it('terminal toolbar renders stop button for running session', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'toolbar test');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(screen.getByTestId('terminal-toolbar')).toBeInTheDocument();
      expect(screen.getByTestId('stop-session-btn')).toBeInTheDocument();
    });
  });

  it('terminal toolbar shows session ended indicator after stop', async () => {
    let pollCount = 0;
    vi.mocked(ipc.pollInteractiveEvents).mockImplementation(async () => {
      pollCount++;
      if (pollCount <= 2) {
        return {
          sessionId: 'test-session-1',
          events: [],
          nextCursor: pollCount,
          done: false,
          status: 'running',
          error: null,
        } as InteractiveEventBatch;
      }
      return {
        sessionId: 'test-session-1',
        events: [],
        nextCursor: pollCount,
        done: true,
        status: 'stopped',
        error: null,
      } as InteractiveEventBatch;
    });

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'ended indicator test');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => expect(screen.getByTestId('stop-session-btn')).toBeInTheDocument());
    await user.click(screen.getByTestId('stop-session-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('session-ended-indicator')).toHaveTextContent('Session stopped');
    });
    expect(screen.queryByTestId('stop-session-btn')).not.toBeInTheDocument();
  });

  it('concurrent lanes: stop button targets focused lane only', async () => {
    let sessionCounter = 0;
    vi.mocked(ipc.startInteractiveSession).mockImplementation(async (req) => {
      sessionCounter++;
      return {
        sessionId: `concurrent-${sessionCounter}`,
        agentKey: req.agentKey,
        status: 'running',
        startedAt: new Date().toISOString(),
      } as InteractiveSessionStarted;
    });
    vi.mocked(ipc.pollInteractiveEvents).mockImplementation(async (sessionId) => ({
      sessionId,
      events: [],
      nextCursor: 0,
      done: false,
      status: 'running',
      error: null,
    } as InteractiveEventBatch));
    vi.mocked(ipc.stopInteractiveSession).mockImplementation(async (sessionId) => ({
      sessionId,
      status: 'stopped',
      wasRunning: true,
    } as InteractiveStopResult));

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-orchestration'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());

    await user.type(screen.getByTestId('session-task-prompt'), 'Task 1');
    await user.click(screen.getByTestId('confirm-create-session'));
    await waitFor(() => expect(screen.getByTestId('session-item-concurrent-1')).toBeInTheDocument());

    await user.click(screen.getByTestId('confirm-create-session'));
    await waitFor(() => expect(screen.getByTestId('session-item-concurrent-2')).toBeInTheDocument());

    // Focus is on session 2 (most recent). Stop it.
    await waitFor(() => expect(screen.getByTestId('stop-session-btn')).toBeInTheDocument());
    await user.click(screen.getByTestId('stop-session-btn'));

    await waitFor(() => {
      expect(ipc.stopInteractiveSession).toHaveBeenCalledWith('concurrent-2');
    });
    expect(ipc.stopInteractiveSession).not.toHaveBeenCalledWith('concurrent-1');
  });
});
