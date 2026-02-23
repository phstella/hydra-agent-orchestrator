import { useState, useCallback, useEffect, useMemo } from 'react';
import { PreflightDashboard } from './components/PreflightDashboard';
import { ExperimentalAdapterModal } from './components/ExperimentalAdapterModal';
import { AgentRail } from './components/AgentRail';
import { LiveOutputPanel } from './components/LiveOutputPanel';
import { ResultsScoreboard } from './components/ResultsScoreboard';
import { Tabs, Badge, Button, Card } from './components/design-system';
import { getRaceResult, listAdapters, pollRaceEvents, startRace } from './ipc';
import type { AdapterInfo, RaceResult } from './types';
import { isExperimental, isTier1 } from './types';
import { useEventBuffer, useAgentStatuses } from './hooks';

const NAV_TABS = [
  { id: 'preflight', label: 'Preflight' },
  { id: 'race', label: 'Race' },
  { id: 'results', label: 'Results' },
];

export default function App() {
  const [activeTab, setActiveTab] = useState('preflight');
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

  const selectedExperimentalCount = useMemo(() => {
    return selectedAdapters.filter((key) => {
      const adapter = adapters.find((a) => a.key === key);
      return !!adapter && isExperimental(adapter);
    }).length;
  }, [adapters, selectedAdapters]);

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
      });
      setActiveRunId(started.runId);
      setRaceAgents(started.agents);
      setSelectedAgent(started.agents[0] ?? null);
      setRunStatus('running');
      setActiveTab('race');
    } catch (err) {
      setRunStatus('failed');
      setRaceError(err instanceof Error ? err.message : String(err));
    }
  }, [clear, selectedAdapters, selectedExperimentalCount, taskPrompt]);

  const handleWinnerSelect = useCallback((agentKey: string) => {
    setSelectedWinner(agentKey);
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

  return (
    <div style={{ minHeight: '100vh', display: 'flex', flexDirection: 'column' }}>
      <header
        style={{
          backgroundColor: 'var(--color-bg-900)',
          borderBottom: '1px solid var(--color-border-700)',
          padding: '0 var(--space-6)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          height: 52,
          flexShrink: 0,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-4)' }}>
          <span
            style={{
              fontSize: 'var(--text-lg)',
              fontWeight: 'var(--weight-bold)' as unknown as number,
              color: 'var(--color-green-400)',
              fontFamily: 'var(--font-mono)',
            }}
          >
            ⟁ Hydra
          </span>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-3)' }}>
          <Badge variant="neutral">{selectedAdapters.length} selected</Badge>
          {selectedExperimentalCount > 0 && (
            <Badge variant="experimental">{selectedExperimentalCount} experimental</Badge>
          )}
          <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>
            v0.1.0-alpha
          </span>
        </div>
      </header>

      <Tabs tabs={NAV_TABS} activeTab={activeTab} onTabChange={setActiveTab}>
        <main style={{ flex: 1 }}>
          {activeTab === 'preflight' && <PreflightDashboard />}

          {activeTab === 'race' && (
            <div style={{ maxWidth: 980, margin: '0 auto', padding: 'var(--space-8) var(--space-6)' }}>
              <Card padding="lg" style={{ marginBottom: 'var(--space-6)' }}>
                <h2
                  style={{
                    fontSize: 'var(--text-xl)',
                    fontWeight: 'var(--weight-bold)' as unknown as number,
                    marginBottom: 'var(--space-3)',
                  }}
                >
                  Start Race
                </h2>

                {adapterLoadError && (
                  <div style={{ marginBottom: 'var(--space-4)', color: 'var(--color-danger-400)' }}>
                    Adapter load failed: {adapterLoadError}
                  </div>
                )}

                <div style={{ marginBottom: 'var(--space-4)' }}>
                  <div style={{ marginBottom: 'var(--space-2)', fontSize: 'var(--text-sm)', color: 'var(--color-text-secondary)' }}>
                    Adapters
                  </div>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: 'var(--space-2)' }}>
                    {adapters.map((adapter) => {
                      const selected = selectedAdapters.includes(adapter.key);
                      const experimental = isExperimental(adapter);

                      return (
                        <button
                          key={adapter.key}
                          type="button"
                          onClick={() => toggleAdapter(adapter.key)}
                          style={{
                            display: 'inline-flex',
                            alignItems: 'center',
                            gap: 'var(--space-2)',
                            borderRadius: 'var(--radius-md)',
                            border: selected
                              ? '1px solid var(--color-marine-500)'
                              : '1px solid var(--color-border-700)',
                            backgroundColor: selected
                              ? 'color-mix(in srgb, var(--color-marine-500) 12%, transparent)'
                              : 'var(--color-surface-800)',
                            color: 'var(--color-text-primary)',
                            padding: 'var(--space-2) var(--space-3)',
                            cursor: 'pointer',
                            fontFamily: 'var(--font-family)',
                            fontSize: 'var(--text-sm)',
                          }}
                        >
                          <span>{adapter.key}</span>
                          {experimental ? (
                            <Badge variant="experimental">Experimental</Badge>
                          ) : (
                            <Badge variant="success">Tier-1</Badge>
                          )}
                        </button>
                      );
                    })}
                  </div>
                </div>

                <div style={{ marginBottom: 'var(--space-4)' }}>
                  <div style={{ marginBottom: 'var(--space-2)', fontSize: 'var(--text-sm)', color: 'var(--color-text-secondary)' }}>
                    Prompt
                  </div>
                  <textarea
                    value={taskPrompt}
                    onChange={(e) => setTaskPrompt(e.target.value)}
                    placeholder="Describe the task for this race..."
                    rows={5}
                    style={{
                      width: '100%',
                      borderRadius: 'var(--radius-md)',
                      border: '1px solid var(--color-border-700)',
                      backgroundColor: 'var(--color-bg-900)',
                      color: 'var(--color-text-primary)',
                      padding: 'var(--space-3)',
                      resize: 'vertical',
                      fontFamily: 'var(--font-family)',
                      fontSize: 'var(--text-sm)',
                    }}
                  />
                </div>

                <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-3)' }}>
                  <Button variant="primary" onClick={handleStartRace}>
                    Start Race
                  </Button>
                  <Badge variant={runStatus === 'failed' ? 'danger' : runStatus === 'completed' ? 'success' : 'info'}>
                    {runStatus}
                  </Badge>
                  {activeRunId && <Badge variant="neutral">run {activeRunId.slice(0, 8)}</Badge>}
                </div>

                {raceError && (
                  <div style={{ marginTop: 'var(--space-3)', color: 'var(--color-danger-400)', fontSize: 'var(--text-sm)' }}>
                    {raceError}
                  </div>
                )}
              </Card>

              {(runStatus !== 'idle' && runStatus !== 'starting') && (
                <div
                  style={{
                    display: 'flex',
                    border: '1px solid var(--color-border-700)',
                    borderRadius: 'var(--radius-lg)',
                    backgroundColor: 'var(--color-surface-800)',
                    overflow: 'hidden',
                    height: 420,
                  }}
                >
                  <div
                    style={{
                      borderRight: '1px solid var(--color-border-700)',
                      padding: 'var(--space-3)',
                      overflowY: 'auto',
                      flexShrink: 0,
                    }}
                  >
                    <AgentRail
                      agents={agentStatuses}
                      selectedAgent={selectedAgent}
                      onSelectAgent={setSelectedAgent}
                    />
                  </div>
                  <LiveOutputPanel
                    agentKey={selectedAgent}
                    lifecycle={agentStatuses.find((a) => a.agentKey === selectedAgent)?.lifecycle ?? null}
                    events={events}
                    eventsByAgent={eventsByAgent}
                  />
                </div>
              )}

              {raceResult && (
                <Card padding="lg" style={{ marginTop: 'var(--space-6)' }}>
                  <h4 style={{ marginBottom: 'var(--space-2)' }}>Race Result</h4>
                  <div style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-secondary)' }}>
                    Status: {raceResult.status}
                  </div>
                  <div style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-secondary)' }}>
                    Agents: {raceResult.agents.length}
                    {selectedWinner && <> · Winner: <strong style={{ color: 'var(--color-green-400)' }}>{selectedWinner}</strong></>}
                  </div>
                  <Button
                    variant="secondary"
                    size="sm"
                    style={{ marginTop: 'var(--space-3)' }}
                    onClick={() => setActiveTab('results')}
                  >
                    View Scoreboard
                  </Button>
                </Card>
              )}
            </div>
          )}

          {activeTab === 'results' && (
            raceResult ? (
              <ResultsScoreboard
                result={raceResult}
                onSelectWinner={handleWinnerSelect}
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
            )
          )}
        </main>
      </Tabs>

      <ExperimentalAdapterModal
        open={experimentalModal.open}
        onClose={closeExperimentalModal}
        onConfirm={handleExperimentalConfirm}
        adapter={experimentalModal.adapter}
      />
    </div>
  );
}
