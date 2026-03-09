import { useState, useCallback, useEffect, useMemo } from 'react';
import type { CSSProperties, ReactNode } from 'react';
import { CockpitShell, NavRailButton, TopStrip } from './components/CockpitShell';
import { CockpitCenter } from './components/CockpitCenter';
import { LeaderboardRail } from './components/LeaderboardRail';
import { PreflightDashboard } from './components/PreflightDashboard';
import { ExperimentalAdapterModal } from './components/ExperimentalAdapterModal';
import { ResultsScoreboard } from './components/ResultsScoreboard';
import { CandidateDiffReview } from './components/CandidateDiffReview';
import {
  InteractiveWorkspace,
  type InteractiveWorkspaceSessionSnapshot,
} from './components/InteractiveWorkspace';
import { FileExplorer } from './components/FileExplorer';
import { Card, Button, Badge } from './components/design-system';
import { getRaceResult, listAdapters, pollRaceEvents, startRace } from './ipc';
import type { AdapterInfo, RaceResult } from './types';
import { isExperimental, isTier1 } from './types';
import { useEventBuffer, useAgentStatuses } from './hooks';

type CockpitView = 'cockpit' | 'preflight' | 'results' | 'review' | 'orchestration' | 'files' | 'settings';

const WORKSPACE_STORAGE_KEY = 'hydra.workspace.path';

type StorageLike = {
  getItem?: (key: string) => string | null;
  setItem?: (key: string, value: string) => void;
  removeItem?: (key: string) => void;
};

function RailSectionHeader({
  label,
  testId,
}: {
  label: string;
  testId: string;
}) {
  const wrapperStyle: CSSProperties = {
    width: '100%',
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    gap: '2px',
    marginBottom: 'var(--space-1)',
  };

  const labelStyle: CSSProperties = {
    fontSize: '10px',
    textTransform: 'none',
    letterSpacing: '0.08em',
    color: 'var(--color-text-secondary)',
    textAlign: 'center',
    lineHeight: 1.1,
    width: '100%',
    padding: '0 4px',
    fontWeight: 'var(--weight-semibold)' as unknown as number,
    fontFamily: 'var(--font-mono)',
    whiteSpace: 'normal',
    overflowWrap: 'anywhere',
    wordBreak: 'break-word',
  };

  return (
    <div style={wrapperStyle}>
      <div style={labelStyle} data-testid={testId}>
        {label}
      </div>
      <div
        style={{
          width: '76%',
          borderTop: '1px solid var(--color-border-600)',
        }}
      />
    </div>
  );
}

function NavGlyph({ children }: { children: ReactNode }) {
  return (
    <svg
      viewBox="0 0 16 16"
      width="16"
      height="16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.4"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

function OrchestrationIcon() {
  return (
    <NavGlyph>
      <rect x="1.8" y="2.4" width="12.4" height="11.2" rx="2" />
      <path d="M4.5 6.1 6.9 8l-2.4 1.9" />
      <path d="M8.7 10h2.9" />
    </NavGlyph>
  );
}

function FilesIcon() {
  return (
    <NavGlyph>
      <path d="M2.1 5.1h11.8l-1 7.2H3.1L2.1 5.1Z" />
      <path d="M2.1 5.1V3.4c0-.8.6-1.4 1.4-1.4h2.7l1.1 1.3h4.9c.8 0 1.4.6 1.4 1.4v.4" />
    </NavGlyph>
  );
}

function RaceIcon() {
  return (
    <NavGlyph>
      <path d="M2.4 12.6V3.3m0 0h7.9l-1.9 2 1.9 2H2.4m4.1 5.3h6.9m-6.9-2.4h6.9" />
    </NavGlyph>
  );
}

function ResultsIcon() {
  return (
    <NavGlyph>
      <path d="M2.3 13.2h11.4" />
      <rect x="3.1" y="7.7" width="2.2" height="3.8" />
      <rect x="6.9" y="5.6" width="2.2" height="5.9" />
      <rect x="10.7" y="3.7" width="2.2" height="7.8" />
    </NavGlyph>
  );
}

function ReviewIcon() {
  return (
    <NavGlyph>
      <path d="M3 2.5h10v11H3z" />
      <path d="M5.2 5.2h5.6M5.2 7.4h5.6M5.2 9.6h3.6" />
      <path d="m10.6 10.2 1 1 1.9-2" />
    </NavGlyph>
  );
}

function DoctorIcon() {
  return (
    <NavGlyph>
      <path d="M8 2.2 3.2 4.1v3.4c0 3.1 2 5 4.8 6.3 2.8-1.3 4.8-3.2 4.8-6.3V4.1L8 2.2Z" />
      <path d="M8 5.2v4.6M5.7 7.5h4.6" />
    </NavGlyph>
  );
}

function SettingsIcon() {
  return (
    <NavGlyph>
      <path d="M3.2 4.1h9.6M3.2 8h9.6M3.2 11.9h9.6" />
      <circle cx="5.2" cy="4.1" r="1.2" />
      <circle cx="10.8" cy="8" r="1.2" />
      <circle cx="7.2" cy="11.9" r="1.2" />
    </NavGlyph>
  );
}

function getStorage(): StorageLike | null {
  if (typeof window === 'undefined') return null;
  return (window as unknown as { localStorage?: StorageLike }).localStorage ?? null;
}

function readWorkspaceFromStorage(): string {
  const storage = getStorage();
  if (!storage || typeof storage.getItem !== 'function') return '';
  return storage.getItem(WORKSPACE_STORAGE_KEY) ?? '';
}

function writeWorkspaceToStorage(path: string): void {
  const storage = getStorage();
  if (!storage) return;

  if (path.length > 0) {
    if (typeof storage.setItem === 'function') {
      storage.setItem(WORKSPACE_STORAGE_KEY, path);
    }
    return;
  }

  if (typeof storage.removeItem === 'function') {
    storage.removeItem(WORKSPACE_STORAGE_KEY);
  }
}

export default function App() {
  const [activeView, setActiveView] = useState<CockpitView>('orchestration');
  const [adapters, setAdapters] = useState<AdapterInfo[]>([]);
  const [adapterLoadError, setAdapterLoadError] = useState<string | null>(null);
  const [selectedAdapters, setSelectedAdapters] = useState<string[]>([]);

  const [experimentalModal, setExperimentalModal] = useState<{
    open: boolean;
    adapter: AdapterInfo | null;
  }>({ open: false, adapter: null });

  const [taskPrompt, setTaskPrompt] = useState('');
  const [activeRunId, setActiveRunId] = useState<string | null>(null);
  const [runStatus, setRunStatus] = useState<string>('idle');
  const [raceError, setRaceError] = useState<string | null>(null);
  const [raceResult, setRaceResult] = useState<RaceResult | null>(null);
  const [selectedWinner, setSelectedWinner] = useState<string | null>(null);
  const [workspacePath, setWorkspacePath] = useState('');
  const [workspaceDraft, setWorkspaceDraft] = useState('');

  const { events, push, clear, eventsByAgent } = useEventBuffer();

  const [raceAgents, setRaceAgents] = useState<string[]>([]);
  const [selectedAgent, setSelectedAgent] = useState<string | null>(null);
  const [orchestrationSessions, setOrchestrationSessions] = useState<InteractiveWorkspaceSessionSnapshot['sessions']>([]);
  const [orchestrationSelectedSessionId, setOrchestrationSelectedSessionId] = useState<string | null>(null);
  const [orchestrationSelectionRequestId, setOrchestrationSelectionRequestId] = useState<string | null>(null);

  const agentStatuses = useAgentStatuses(events, raceAgents, runStatus);

  useEffect(() => {
    let cancelled = false;

    async function loadAdapters() {
      try {
        const data = await listAdapters();
        if (cancelled) return;

        setAdapters(data);
        setAdapterLoadError(null);

        const tier1Defaults = data
          .filter((adapter) => isTier1(adapter) && adapter.status === 'ready')
          .map((adapter) => adapter.key);
        setSelectedAdapters(tier1Defaults);
      } catch (err) {
        if (cancelled) return;
        setAdapterLoadError(err instanceof Error ? err.message : String(err));
      }
    }

    loadAdapters();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const stored = readWorkspaceFromStorage();
    setWorkspacePath(stored);
    setWorkspaceDraft(stored);
  }, []);

  const selectedExperimentalCount = useMemo(() => {
    return selectedAdapters.filter((key) => {
      const adapter = adapters.find((a) => a.key === key);
      return !!adapter && isExperimental(adapter);
    }).length;
  }, [adapters, selectedAdapters]);

  const workspaceCwd = useMemo(() => {
    const trimmed = workspacePath.trim();
    return trimmed.length > 0 ? trimmed : null;
  }, [workspacePath]);

  const workspaceDraftCwd = useMemo(() => {
    const trimmed = workspaceDraft.trim();
    return trimmed.length > 0 ? trimmed : null;
  }, [workspaceDraft]);

  const workspaceDirty = workspaceDraft.trim() !== workspacePath.trim();
  const activeThreadCount = useMemo(() => (
    orchestrationSessions.filter((session) => session.status === 'running').length
  ), [orchestrationSessions]);

  const topStripThreadOptions = useMemo(() => (
    orchestrationSessions.map((session) => ({
      sessionId: session.sessionId,
      label: `${session.agentKey} · ${session.sessionId.slice(0, 8)} · ${session.status}`,
    }))
  ), [orchestrationSessions]);

  const topStripSelectedThreadId = orchestrationSelectionRequestId ?? orchestrationSelectedSessionId;

  const openExperimentalModal = useCallback((adapter: AdapterInfo) => {
    setExperimentalModal({ open: true, adapter });
  }, []);

  const closeExperimentalModal = useCallback(() => {
    setExperimentalModal({ open: false, adapter: null });
  }, []);

  const handleExperimentalConfirm = useCallback(() => {
    const adapter = experimentalModal.adapter;
    if (!adapter) return;
    setSelectedAdapters((prev) => (prev.includes(adapter.key) ? prev : [...prev, adapter.key]));
    setExperimentalModal({ open: false, adapter: null });
  }, [experimentalModal.adapter]);

  const toggleAdapter = useCallback(
    (adapterKey: string) => {
      const adapter = adapters.find((a) => a.key === adapterKey);
      if (!adapter) return;

      setSelectedAdapters((prev) => {
        if (prev.includes(adapterKey)) {
          return prev.filter((key) => key !== adapterKey);
        }

        if (isExperimental(adapter)) {
          openExperimentalModal(adapter);
          return prev;
        }

        return [...prev, adapterKey];
      });
    },
    [adapters, openExperimentalModal],
  );

  const handleStartRace = useCallback(async () => {
    if (!taskPrompt.trim()) {
      setRaceError('Enter a task prompt before starting a race.');
      return;
    }
    if (selectedAdapters.length === 0) {
      setRaceError('Select at least one adapter.');
      return;
    }

    setRaceError(null);
    setRaceResult(null);
    setSelectedWinner(null);
    setRunStatus('starting');
    setActiveRunId(null);
    setRaceAgents([]);
    setSelectedAgent(null);
    clear();

    try {
      const started = await startRace({
        taskPrompt,
        agents: selectedAdapters,
        allowExperimental: selectedExperimentalCount > 0,
        cwd: workspaceCwd,
      });
      setActiveRunId(started.runId);
      setRaceAgents(started.agents);
      setSelectedAgent(started.agents[0] ?? null);
      setRunStatus('running');
      setActiveView('cockpit');
    } catch (err) {
      setRunStatus('failed');
      setRaceError(err instanceof Error ? err.message : String(err));
    }
  }, [clear, selectedAdapters, selectedExperimentalCount, taskPrompt, workspaceCwd]);

  const handleWinnerSelect = useCallback((agentKey: string) => {
    setSelectedWinner(agentKey);
  }, []);

  const handleWinnerSelectAndReview = useCallback((agentKey: string) => {
    setSelectedWinner(agentKey);
    setActiveView('review');
  }, []);

  const handleOpenReview = useCallback((agentKey: string) => {
    setSelectedWinner(agentKey);
    setActiveView('review');
  }, []);

  const handlePrepareNewRace = useCallback(() => {
    setRunStatus('idle');
    setRaceError(null);
    setRaceResult(null);
    setSelectedWinner(null);
    setActiveRunId(null);
    setRaceAgents([]);
    setSelectedAgent(null);
    clear();
    setActiveView('cockpit');
  }, [clear]);

  const handleOrchestrationSnapshotChange = useCallback((snapshot: InteractiveWorkspaceSessionSnapshot) => {
    setOrchestrationSessions(snapshot.sessions);
    setOrchestrationSelectedSessionId(snapshot.selectedSessionId);
    setOrchestrationSelectionRequestId((current) => {
      if (!current) return null;
      if (snapshot.selectedSessionId === current) return null;
      if (snapshot.sessions.length > 0 && !snapshot.sessions.some((session) => session.sessionId === current)) {
        return null;
      }
      return current;
    });
  }, []);

  const handleTopStripThreadSelect = useCallback((sessionId: string) => {
    setActiveView('orchestration');
    setOrchestrationSelectionRequestId(sessionId);
  }, []);

  const handleSaveWorkspaceSettings = useCallback(() => {
    const normalized = workspaceDraft.trim();
    setWorkspacePath(normalized);
    writeWorkspaceToStorage(normalized);
  }, [workspaceDraft]);

  const handleResetWorkspaceSettings = useCallback(() => {
    setWorkspacePath('');
    setWorkspaceDraft('');
    writeWorkspaceToStorage('');
  }, []);

  useEffect(() => {
    if (!activeRunId) return;
    const runId = activeRunId;

    let cancelled = false;
    let cursor = 0;

    async function poll() {
      if (cancelled) return;
      try {
        const batch = await pollRaceEvents(runId, cursor);
        cursor = batch.nextCursor;

        for (const evt of batch.events) {
          push(evt);
        }

        if (batch.error) {
          setRaceError(batch.error);
        }

        setRunStatus(batch.status);

        if (batch.done) {
          const result = await getRaceResult(runId);
          setRaceResult(result);
          if (result) {
            setRunStatus(result.status);
          }
          return;
        }
      } catch (err) {
        setRaceError(err instanceof Error ? err.message : String(err));
        setRunStatus('failed');
        return;
      }

      setTimeout(poll, 250);
    }

    poll();

    return () => {
      cancelled = true;
    };
  }, [activeRunId, push]);

  const railGroupStyle: CSSProperties = {
    width: '100%',
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'stretch',
    gap: '2px',
    padding: 'var(--space-2) 2px',
    marginBottom: 'var(--space-2)',
    border: '1px solid var(--color-border-700)',
    borderRadius: 'var(--radius-md)',
    backgroundColor: 'color-mix(in srgb, var(--color-bg-950) 78%, var(--color-marine-500) 6%)',
  };

  const leftRail = (
    <>
      <div style={railGroupStyle}>
        <RailSectionHeader label="Orchestration" testId="nav-group-orchestration-label" />
        <NavRailButton
          icon={<OrchestrationIcon />}
          label="Orchestration"
          active={activeView === 'orchestration'}
          onClick={() => setActiveView('orchestration')}
          data-testid="nav-orchestration"
        />
        <NavRailButton
          icon={<FilesIcon />}
          label="Files"
          active={activeView === 'files'}
          onClick={() => setActiveView('files')}
          data-testid="nav-files"
        />
      </div>

      <div style={railGroupStyle}>
        <RailSectionHeader label="Race" testId="nav-group-race-label" />
        <NavRailButton
          icon={<RaceIcon />}
          label="Race"
          active={activeView === 'cockpit'}
          onClick={() => setActiveView('cockpit')}
          data-testid="nav-cockpit"
        />
        <NavRailButton
          icon={<ResultsIcon />}
          label="Results"
          active={activeView === 'results'}
          onClick={() => setActiveView('results')}
          data-testid="nav-results"
        />
        <NavRailButton
          icon={<ReviewIcon />}
          label="Review"
          active={activeView === 'review'}
          onClick={() => setActiveView('review')}
          data-testid="nav-review"
        />
      </div>

      <div style={{ flex: 1 }} />
      <div style={railGroupStyle}>
        <RailSectionHeader label="System" testId="nav-group-system-label" />
        <NavRailButton
          icon={<DoctorIcon />}
          label={'Hydra\nDoctor'}
          active={activeView === 'preflight'}
          onClick={() => setActiveView('preflight')}
          data-testid="nav-preflight"
        />
        <NavRailButton
          icon={<SettingsIcon />}
          label="Settings"
          active={activeView === 'settings'}
          onClick={() => setActiveView('settings')}
          data-testid="nav-settings"
        />
      </div>
    </>
  );

  const topStrip = (
    <TopStrip
      workspacePath={workspaceCwd}
      runStatus={runStatus}
      runId={activeRunId}
      adapterCount={selectedAdapters.length}
      experimentalCount={selectedExperimentalCount}
      activeThreadCount={activeThreadCount}
      threadOptions={topStripThreadOptions}
      selectedThreadId={topStripSelectedThreadId}
      onSelectThread={handleTopStripThreadSelect}
    />
  );

  const rightRail = activeView === 'cockpit'
    ? (
        <LeaderboardRail
          agents={agentStatuses}
          raceResult={raceResult}
          selectedAgent={selectedAgent}
          onSelectAgent={setSelectedAgent}
          raceError={raceError}
        />
      )
    : null;

  const renderCenter = () => {
    switch (activeView) {
      case 'cockpit':
        return (
          <CockpitCenter
            adapters={adapters}
            adapterLoadError={adapterLoadError}
            selectedAdapters={selectedAdapters}
            onToggleAdapter={toggleAdapter}
            taskPrompt={taskPrompt}
            onTaskPromptChange={setTaskPrompt}
            workspaceCwd={workspaceCwd}
            onOpenSettings={() => setActiveView('settings')}
            onStartRace={handleStartRace}
            runStatus={runStatus}
            raceError={raceError}
            events={events}
            eventsByAgent={eventsByAgent}
            agentStatuses={agentStatuses}
            selectedAgent={selectedAgent}
            raceResult={raceResult}
            selectedWinner={selectedWinner}
            onSelectWinner={handleWinnerSelect}
            onOpenReview={handleOpenReview}
            onOpenOrchestration={() => setActiveView('orchestration')}
            onStartNewRace={handlePrepareNewRace}
          />
        );

      case 'preflight':
        return <PreflightDashboard />;

      case 'results':
        return raceResult ? (
          <ResultsScoreboard
            result={raceResult}
            selectedWinner={selectedWinner}
            onSelectWinner={handleWinnerSelectAndReview}
          />
        ) : (
          <div
            style={{
              padding: 'var(--space-8)',
              textAlign: 'center',
              color: 'var(--color-text-muted)',
            }}
          >
            No results yet. Complete a race to see the scoreboard.
          </div>
        );

      case 'review':
        return raceResult && activeRunId ? (
          <CandidateDiffReview
            runId={activeRunId}
            agents={raceResult.agents}
            selectedWinner={selectedWinner}
            workspaceCwd={workspaceCwd}
          />
        ) : (
          <div
            style={{
              padding: 'var(--space-8)',
              textAlign: 'center',
              color: 'var(--color-text-muted)',
            }}
          >
            No results yet. Complete a race and select a winner to review diffs.
          </div>
        );

      case 'orchestration':
        return (
          <InteractiveWorkspace
            workspaceCwd={workspaceCwd}
            selectedSessionIdOverride={orchestrationSelectionRequestId}
            onSessionSnapshotChange={handleOrchestrationSnapshotChange}
          />
        );

      case 'files':
        return <FileExplorer workspaceCwd={workspaceCwd} />;

      case 'settings':
        return (
          <div style={{ maxWidth: 920, margin: '0 auto', padding: 'var(--space-8) var(--space-6)' }}>
            <Card padding="lg">
              <h2
                style={{
                  fontSize: 'var(--text-xl)',
                  fontWeight: 'var(--weight-bold)' as unknown as number,
                  marginBottom: 'var(--space-2)',
                }}
              >
                Settings
              </h2>
              <p style={{ marginBottom: 'var(--space-5)', color: 'var(--color-text-secondary)', fontSize: 'var(--text-sm)' }}>
                Configure default workspace and runtime behavior used by race, review/merge, and orchestration sessions.
              </p>

              <div style={{ marginBottom: 'var(--space-4)' }}>
                <div style={{ marginBottom: 'var(--space-2)', fontSize: 'var(--text-sm)', color: 'var(--color-text-secondary)' }}>
                  Default Workspace Folder
                </div>
                <input
                  value={workspaceDraft}
                  onChange={(e) => setWorkspaceDraft(e.target.value)}
                  placeholder="Leave empty to use current repository (or enter /absolute/path)"
                  data-testid="settings-workspace-input"
                  style={{
                    width: '100%',
                    borderRadius: 'var(--radius-md)',
                    border: '1px solid var(--color-border-700)',
                    backgroundColor: 'var(--color-bg-900)',
                    color: 'var(--color-text-primary)',
                    padding: 'var(--space-3)',
                    fontFamily: 'var(--font-family)',
                    fontSize: 'var(--text-sm)',
                  }}
                />
                <div style={{ marginTop: 'var(--space-1)', fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>
                  Current effective workspace: {workspaceCwd ?? '(current repository)'}
                </div>
              </div>

              <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-3)' }}>
                <Button
                  variant="primary"
                  onClick={handleSaveWorkspaceSettings}
                  disabled={!workspaceDirty}
                  data-testid="settings-save-workspace"
                >
                  Save Workspace
                </Button>
                <Button variant="ghost" onClick={handleResetWorkspaceSettings} data-testid="settings-reset-workspace">
                  Reset to Current Repository
                </Button>
                <Badge variant={workspaceDraftCwd ? 'info' : 'neutral'}>
                  {workspaceDraftCwd ?? '(current repository)'}
                </Badge>
              </div>
            </Card>
          </div>
        );
    }
  };

  return (
    <>
      <CockpitShell
        leftRail={leftRail}
        topStrip={topStrip}
        center={renderCenter()}
        rightRail={rightRail}
      />
      <ExperimentalAdapterModal
        open={experimentalModal.open}
        onClose={closeExperimentalModal}
        onConfirm={handleExperimentalConfirm}
        adapter={experimentalModal.adapter}
      />
    </>
  );
}
