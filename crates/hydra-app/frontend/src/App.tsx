import { useState, useCallback, useEffect, useMemo } from 'react';
import { CockpitShell, NavRailButton, TopStrip } from './components/CockpitShell';
import { CockpitCenter } from './components/CockpitCenter';
import { LeaderboardRail } from './components/LeaderboardRail';
import { PreflightDashboard } from './components/PreflightDashboard';
import { ExperimentalAdapterModal } from './components/ExperimentalAdapterModal';
import { ResultsScoreboard } from './components/ResultsScoreboard';
import { CandidateDiffReview } from './components/CandidateDiffReview';
import { InteractiveWorkspace } from './components/InteractiveWorkspace';
import { Card, Button, Badge } from './components/design-system';
import { getRaceResult, listAdapters, pollRaceEvents, startRace } from './ipc';
import type { AdapterInfo, RaceResult } from './types';
import { isExperimental, isTier1 } from './types';
import { useEventBuffer, useAgentStatuses } from './hooks';

type CockpitView = 'cockpit' | 'preflight' | 'results' | 'review' | 'interactive' | 'settings';

const WORKSPACE_STORAGE_KEY = 'hydra.workspace.path';

type StorageLike = {
  getItem?: (key: string) => string | null;
  setItem?: (key: string, value: string) => void;
  removeItem?: (key: string) => void;
};

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
  const [activeView, setActiveView] = useState<CockpitView>('cockpit');
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
  const [interventionError, setInterventionError] = useState<string | null>(null);

  const { events, push, clear, eventsByAgent } = useEventBuffer();

  const [raceAgents, setRaceAgents] = useState<string[]>([]);
  const [selectedAgent, setSelectedAgent] = useState<string | null>(null);

  const agentStatuses = useAgentStatuses(events, raceAgents);

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
    setInterventionError(null);
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

  const handleOpenReview = useCallback(() => {
    setActiveView('review');
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

  const handleSendInput = useCallback(
    async (_input: string): Promise<{ success: boolean; error: string | null }> => {
      setInterventionError(null);
      return { success: false, error: 'Intervention during race mode is not supported yet' };
    },
    [],
  );

  const handleStopAgent = useCallback(() => {
    setInterventionError('Stop during race not yet implemented. Use interrupt from CLI.');
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

  const leftRail = (
    <>
      <NavRailButton
        icon="⟁"
        label="Race"
        active={activeView === 'cockpit'}
        onClick={() => setActiveView('cockpit')}
        data-testid="nav-cockpit"
      />
      <NavRailButton
        icon="◉"
        label="Preflight"
        active={activeView === 'preflight'}
        onClick={() => setActiveView('preflight')}
        data-testid="nav-preflight"
      />
      <NavRailButton
        icon="≡"
        label="Results"
        active={activeView === 'results'}
        onClick={() => setActiveView('results')}
        data-testid="nav-results"
      />
      <NavRailButton
        icon="⊟"
        label="Review"
        active={activeView === 'review'}
        onClick={() => setActiveView('review')}
        data-testid="nav-review"
      />
      <NavRailButton
        icon="▸"
        label="Terminal"
        active={activeView === 'interactive'}
        onClick={() => setActiveView('interactive')}
        data-testid="nav-interactive"
      />
      <div style={{ flex: 1 }} />
      <NavRailButton
        icon="⚙"
        label="Settings"
        active={activeView === 'settings'}
        onClick={() => setActiveView('settings')}
        data-testid="nav-settings"
      />
    </>
  );

  const topStrip = (
    <TopStrip
      workspacePath={workspaceCwd}
      runStatus={runStatus}
      runId={activeRunId}
      adapterCount={selectedAdapters.length}
      experimentalCount={selectedExperimentalCount}
      onRun={runStatus === 'idle' ? handleStartRace : undefined}
      onStop={runStatus === 'running' ? handleStopAgent : undefined}
    />
  );

  const rightRail = (
    <LeaderboardRail
      agents={agentStatuses}
      raceResult={raceResult}
      selectedAgent={selectedAgent}
      onSelectAgent={setSelectedAgent}
      raceError={raceError}
    />
  );

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
            activeRunId={activeRunId}
            raceError={raceError}
            events={events}
            eventsByAgent={eventsByAgent}
            agentStatuses={agentStatuses}
            selectedAgent={selectedAgent}
            raceResult={raceResult}
            selectedWinner={selectedWinner}
            onSelectWinner={handleWinnerSelect}
            onOpenReview={handleOpenReview}
            onSendInput={handleSendInput}
            onStopAgent={handleStopAgent}
            interventionError={interventionError}
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

      case 'interactive':
        return <InteractiveWorkspace workspaceCwd={workspaceCwd} />;

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
                Configure default workspace and runtime behavior used by race, review/merge, and interactive sessions.
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
