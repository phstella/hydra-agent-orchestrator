import type { CSSProperties } from 'react';
import { LiveOutputPanel } from './LiveOutputPanel';
import { CompletionSummary } from './CompletionSummary';
import { Badge, Button } from './design-system';
import type { AdapterInfo, AgentStreamEvent, RaceResult } from '../types';
import { isExperimental } from '../types';
import type { AgentStatus } from '../hooks/useAgentStatuses';

interface CockpitCenterProps {
  adapters: AdapterInfo[];
  adapterLoadError: string | null;
  selectedAdapters: string[];
  onToggleAdapter: (key: string) => void;
  taskPrompt: string;
  onTaskPromptChange: (v: string) => void;
  workspaceCwd: string | null;
  onOpenSettings: () => void;
  onStartRace: () => void;
  runStatus: string;
  raceError: string | null;
  events: AgentStreamEvent[];
  eventsByAgent: (agentKey: string) => AgentStreamEvent[];
  agentStatuses: AgentStatus[];
  selectedAgent: string | null;
  raceResult: RaceResult | null;
  selectedWinner: string | null;
  onSelectWinner: (key: string) => void;
  onOpenReview: (agentKey: string) => void;
  onOpenInteractive: () => void;
  onStartNewRace: () => void;
}

export function CockpitCenter({
  adapters,
  adapterLoadError,
  selectedAdapters,
  onToggleAdapter,
  taskPrompt,
  onTaskPromptChange,
  workspaceCwd,
  onOpenSettings,
  onStartRace,
  runStatus,
  raceError,
  events,
  eventsByAgent,
  agentStatuses,
  selectedAgent,
  raceResult,
  selectedWinner,
  onSelectWinner,
  onOpenReview,
  onOpenInteractive,
  onStartNewRace,
}: CockpitCenterProps) {
  const isIdle = runStatus === 'idle';
  const isRunning = runStatus === 'running';
  const isStarting = runStatus === 'starting';
  const isFailed = runStatus === 'failed';
  const isCompleted = runStatus === 'completed';

  const selectedAgentStatus = agentStatuses.find((a) => a.agentKey === selectedAgent) ?? null;
  const selectedAgentLifecycle = selectedAgentStatus?.lifecycle ?? null;

  const showConfig = isIdle || isFailed;
  const showTerminal = isRunning || isStarting || isCompleted;
  const showCompletion = isCompleted && raceResult !== null;

  const containerStyle: CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    flex: 1,
    minHeight: 0,
    overflow: 'hidden',
  };

  return (
    <div style={containerStyle} data-testid="cockpit-center-content">
      {showConfig && (
        <RaceConfigPanel
          adapters={adapters}
          adapterLoadError={adapterLoadError}
          selectedAdapters={selectedAdapters}
          onToggleAdapter={onToggleAdapter}
          taskPrompt={taskPrompt}
          onTaskPromptChange={onTaskPromptChange}
          workspaceCwd={workspaceCwd}
          onOpenSettings={onOpenSettings}
          onStartRace={onStartRace}
          raceError={raceError}
        />
      )}

      {showTerminal && (
        <div style={{ display: 'flex', flexDirection: 'column', flex: 1, minHeight: 0 }}>
          <div style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>
            <LiveOutputPanel
              agentKey={selectedAgent}
              lifecycle={selectedAgentLifecycle}
              events={events}
              eventsByAgent={eventsByAgent}
            />
          </div>

          {isRunning && selectedAgent && selectedAgentLifecycle === 'running' && (
            <div
              style={{
                padding: 'var(--space-3) var(--space-4)',
                fontSize: 'var(--text-xs)',
                color: 'var(--color-text-secondary)',
                backgroundColor: 'var(--color-bg-900)',
                borderTop: '1px solid var(--color-border-700)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                gap: 'var(--space-3)',
              }}
              data-testid="cockpit-intervention-info"
            >
              <span>Race mode is non-interactive. Use Terminal view for mid-flight intervention.</span>
              <Button variant="secondary" size="sm" onClick={onOpenInteractive} data-testid="cockpit-open-interactive">
                Open Terminal View
              </Button>
            </div>
          )}
        </div>
      )}

      {showCompletion && (
        <CompletionSummary
          raceResult={raceResult}
          selectedWinner={selectedWinner}
          onSelectWinner={onSelectWinner}
          onOpenReview={onOpenReview}
          onStartNewRace={onStartNewRace}
        />
      )}

      {raceError && !isIdle && (
        <div
          style={{
            padding: 'var(--space-3) var(--space-4)',
            fontSize: 'var(--text-sm)',
            color: 'var(--color-danger-400)',
            backgroundColor: 'color-mix(in srgb, var(--color-danger-500) 8%, transparent)',
            borderTop: '1px solid var(--color-border-700)',
          }}
          data-testid="cockpit-race-error"
        >
          {raceError}
        </div>
      )}
    </div>
  );
}

interface RaceConfigPanelProps {
  adapters: AdapterInfo[];
  adapterLoadError: string | null;
  selectedAdapters: string[];
  onToggleAdapter: (key: string) => void;
  taskPrompt: string;
  onTaskPromptChange: (v: string) => void;
  workspaceCwd: string | null;
  onOpenSettings: () => void;
  onStartRace: () => void;
  raceError: string | null;
}

function RaceConfigPanel({
  adapters,
  adapterLoadError,
  selectedAdapters,
  onToggleAdapter,
  taskPrompt,
  onTaskPromptChange,
  workspaceCwd,
  onOpenSettings,
  onStartRace,
  raceError,
}: RaceConfigPanelProps) {
  const containerStyle: CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    gap: 'var(--space-4)',
    padding: 'var(--space-6)',
    maxWidth: 800,
    margin: '0 auto',
    width: '100%',
  };

  return (
    <div style={containerStyle} data-testid="race-config-panel">
      <h2
        style={{
          fontSize: 'var(--text-xl)',
          fontWeight: 'var(--weight-bold)' as unknown as number,
          color: 'var(--color-text-primary)',
        }}
      >
        Race Configuration
      </h2>

      {adapterLoadError && (
        <div style={{ color: 'var(--color-danger-400)', fontSize: 'var(--text-sm)' }}>
          Adapter load failed: {adapterLoadError}
        </div>
      )}

      <div
        style={{
          padding: 'var(--space-3)',
          border: '1px solid var(--color-border-700)',
          borderRadius: 'var(--radius-md)',
          backgroundColor: 'var(--color-surface-800)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: 'var(--space-3)',
        }}
      >
        <div>
          <div style={{ marginBottom: 'var(--space-1)', fontSize: 'var(--text-sm)', color: 'var(--color-text-secondary)' }}>
            Workspace
          </div>
          <div style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-primary)' }} data-testid="workspace-current-value">
            {workspaceCwd ?? '(current repository)'}
          </div>
        </div>
        <Button variant="secondary" size="sm" onClick={onOpenSettings}>
          Configure
        </Button>
      </div>

      <div>
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
                onClick={() => onToggleAdapter(adapter.key)}
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

      <div>
        <div style={{ marginBottom: 'var(--space-2)', fontSize: 'var(--text-sm)', color: 'var(--color-text-secondary)' }}>
          Prompt
        </div>
        <textarea
          value={taskPrompt}
          onChange={(e) => onTaskPromptChange(e.target.value)}
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
        <Button variant="primary" onClick={onStartRace} data-testid="cockpit-start-race">
          Start Race
        </Button>
      </div>

      {raceError && (
        <div style={{ color: 'var(--color-danger-400)', fontSize: 'var(--text-sm)' }}>
          {raceError}
        </div>
      )}
    </div>
  );
}
