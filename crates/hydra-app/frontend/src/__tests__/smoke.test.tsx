/**
 * P3-QA-01 + M4.3/M4.4 + M4.7: GUI Smoke Test Pack
 *
 * Covers: cockpit shell render, startup, preflight refresh, experimental modal gating,
 * race flow from cockpit, winner selection, diff candidate switching, merge dry-run gating,
 * interactive tab, session creation, output polling, send input, stop session,
 * leaderboard updates, agent focus switch, intervention, completion summary.
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
  InteractiveWriteAck,
  InteractiveStopResult,
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
  vi.mocked(ipc.stopInteractiveSession).mockResolvedValue({
    sessionId: 'test-session-1',
    status: 'stopped',
    wasRunning: true,
  } as InteractiveStopResult);
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
  it('renders cockpit shell with left rail, top strip, center, and right rail', async () => {
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('cockpit-shell')).toBeInTheDocument();
      expect(screen.getByTestId('cockpit-left-rail')).toBeInTheDocument();
      expect(screen.getByTestId('cockpit-top-strip')).toBeInTheDocument();
      expect(screen.getByTestId('cockpit-center')).toBeInTheDocument();
      expect(screen.getByTestId('cockpit-right-rail')).toBeInTheDocument();
    });
  });

  it('renders navigation buttons in left rail', async () => {
    render(<App />);
    await waitFor(() => {
      expect(screen.getByTestId('nav-cockpit')).toBeInTheDocument();
      expect(screen.getByTestId('nav-preflight')).toBeInTheDocument();
      expect(screen.getByTestId('nav-results')).toBeInTheDocument();
      expect(screen.getByTestId('nav-review')).toBeInTheDocument();
      expect(screen.getByTestId('nav-interactive')).toBeInTheDocument();
      expect(screen.getByTestId('nav-settings')).toBeInTheDocument();
    });
  });

  it('defaults to cockpit view with race config', async () => {
    render(<App />);
    await waitFor(() => {
      expect(screen.getByTestId('race-config-panel')).toBeInTheDocument();
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

    await waitFor(() => {
      expect(screen.getByText('cursor-agent')).toBeInTheDocument();
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

    await waitFor(() => {
      expect(screen.getByText('claude')).toBeInTheDocument();
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

    await waitFor(() => expect(screen.getByText('claude')).toBeInTheDocument());

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

    await waitFor(() => expect(screen.getByText('claude')).toBeInTheDocument());

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

    await waitFor(() => expect(screen.getByText('claude')).toBeInTheDocument());

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

    await waitFor(() => expect(screen.getByText('claude')).toBeInTheDocument());

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

    await waitFor(() => expect(screen.getByText('claude')).toBeInTheDocument());

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

    await waitFor(() => expect(screen.getByText('claude')).toBeInTheDocument());

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
// Interactive Session Smoke Tests (M4.3 + M4.4)
// ---------------------------------------------------------------------------

describe('Smoke Test 8: Interactive tab renders and shows empty state', () => {
  it('renders the Interactive tab in navigation', async () => {
    render(<App />);
    await waitFor(() => {
      expect(screen.getByTestId('nav-interactive')).toBeInTheDocument();
    });
  });

  it('shows empty session state when no sessions exist', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-interactive'));
    await waitFor(() => {
      expect(screen.getByTestId('empty-session-state')).toBeInTheDocument();
    });
    expect(screen.getByTestId('terminal-empty-state')).toBeInTheDocument();
  });
});

describe('Smoke Test 9: Create and select interactive session', () => {
  it('opens new session form and creates session with IPC', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-interactive'));

    await waitFor(() => {
      expect(screen.getByTestId('create-session-btn')).toBeInTheDocument();
    });

    await user.click(screen.getByTestId('create-session-btn'));

    await waitFor(() => {
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
    await user.click(screen.getByTestId('nav-interactive'));

    await waitFor(() => expect(screen.getByTestId('create-session-btn')).toBeInTheDocument());
    await user.click(screen.getByTestId('create-session-btn'));

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

describe('Smoke Test 11: Send input success and failure paths', () => {
  it('sends input successfully when session is running', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-interactive'));

    await waitFor(() => expect(screen.getByTestId('create-session-btn')).toBeInTheDocument());
    await user.click(screen.getByTestId('create-session-btn'));

    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'Test input');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(screen.getByTestId('interactive-input')).toBeInTheDocument();
    });

    const inputField = screen.getByTestId('interactive-input');
    await user.type(inputField, 'do something');

    const sendBtn = screen.getByTestId('send-input-btn');
    await user.click(sendBtn);

    await waitFor(() => {
      expect(ipc.writeInteractiveInput).toHaveBeenCalledWith('test-session-1', 'do something');
    });
  });

  it('shows error feedback when write fails', async () => {
    vi.mocked(ipc.writeInteractiveInput).mockResolvedValue({
      sessionId: 'test-session-1',
      success: false,
      error: 'Session is not running',
    });

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-interactive'));

    await waitFor(() => expect(screen.getByTestId('create-session-btn')).toBeInTheDocument());
    await user.click(screen.getByTestId('create-session-btn'));
    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'Test input err');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => expect(screen.getByTestId('interactive-input')).toBeInTheDocument());

    const inputField = screen.getByTestId('interactive-input');
    await user.type(inputField, 'bad input');
    await user.click(screen.getByTestId('send-input-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('input-error')).toBeInTheDocument();
      expect(screen.getByText(/Session is not running/)).toBeInTheDocument();
    });
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
    await user.click(screen.getByTestId('nav-interactive'));

    await waitFor(() => expect(screen.getByTestId('create-session-btn')).toBeInTheDocument());
    await user.click(screen.getByTestId('create-session-btn'));
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

describe('Smoke Test 13: Interactive terminal handles stream errors and ANSI output', () => {
  it('shows a connection warning when polling fails', async () => {
    vi.mocked(ipc.pollInteractiveEvents).mockRejectedValue(new Error('connection refused'));

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-interactive'));

    await waitFor(() => expect(screen.getByTestId('create-session-btn')).toBeInTheDocument());
    await user.click(screen.getByTestId('create-session-btn'));
    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'Poll fail test');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(screen.getByTestId('terminal-transport-error')).toHaveTextContent('connection refused');
    });
  });

  it('renders ANSI output as plain readable text', async () => {
    vi.mocked(ipc.pollInteractiveEvents).mockResolvedValue({
      sessionId: 'test-session-1',
      events: [
        {
          sessionId: 'test-session-1',
          agentKey: 'claude',
          eventType: 'output',
          data: { text: '\u001b[32mGreen output\u001b[0m\r\n' },
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
    await user.click(screen.getByTestId('nav-interactive'));

    await waitFor(() => expect(screen.getByTestId('create-session-btn')).toBeInTheDocument());
    await user.click(screen.getByTestId('create-session-btn'));
    await waitFor(() => expect(screen.getByTestId('session-task-prompt')).toBeInTheDocument());
    await user.type(screen.getByTestId('session-task-prompt'), 'ANSI test');
    await user.click(screen.getByTestId('confirm-create-session'));

    await waitFor(() => {
      expect(screen.getByText('Green output')).toBeInTheDocument();
    });
    expect(screen.queryByText(/\u001b\[32m/)).not.toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// M4.5: Interactive Safety and Capability Gating Smoke Tests
// ---------------------------------------------------------------------------

describe('Smoke Test 14: Experimental adapter shows warning and requires acknowledgment', () => {
  it('shows experimental warning when selecting cursor-agent', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId('nav-interactive'));

    await waitFor(() => expect(screen.getByTestId('create-session-btn')).toBeInTheDocument());
    await user.click(screen.getByTestId('create-session-btn'));

    await waitFor(() => expect(screen.getByTestId('new-session-form')).toBeInTheDocument());

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
    await user.click(screen.getByTestId('nav-interactive'));

    await waitFor(() => expect(screen.getByTestId('create-session-btn')).toBeInTheDocument());
    await user.click(screen.getByTestId('create-session-btn'));

    await waitFor(() => expect(screen.getByTestId('new-session-form')).toBeInTheDocument());

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
    await user.click(screen.getByTestId('nav-interactive'));

    await waitFor(() => expect(screen.getByTestId('create-session-btn')).toBeInTheDocument());
    await user.click(screen.getByTestId('create-session-btn'));
    await waitFor(() => expect(screen.getByTestId('new-session-form')).toBeInTheDocument());

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
    await user.click(screen.getByTestId('nav-interactive'));

    await waitFor(() => expect(screen.getByTestId('create-session-btn')).toBeInTheDocument());
    await user.click(screen.getByTestId('create-session-btn'));
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
    await user.click(screen.getByTestId('nav-interactive'));

    await waitFor(() => expect(screen.getByTestId('create-session-btn')).toBeInTheDocument());
    await user.click(screen.getByTestId('create-session-btn'));
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

    await waitFor(() => expect(screen.getByText('claude')).toBeInTheDocument());

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

    await waitFor(() => expect(screen.getByText('claude')).toBeInTheDocument());

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

    await waitFor(() => expect(screen.getByText('claude')).toBeInTheDocument());

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
