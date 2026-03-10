import { useState, useMemo } from 'react';
import type { CSSProperties } from 'react';
import { Badge } from './design-system';
import type { AgentLifecycle, AgentStatus } from '../hooks/useAgentStatuses';
import type { AgentResult, RaceResult } from '../types';

interface LeaderboardRailProps {
  agents: AgentStatus[];
  raceResult: RaceResult | null;
  selectedAgent: string | null;
  onSelectAgent: (agentKey: string) => void;
  raceError: string | null;
}

const lifecycleBadgeVariant: Record<AgentLifecycle, 'info' | 'success' | 'danger' | 'warning'> = {
  running: 'info',
  completed: 'success',
  failed: 'danger',
  timed_out: 'warning',
};

const lifecycleLabel: Record<AgentLifecycle, string> = {
  running: 'Running',
  completed: 'Completed',
  failed: 'Failed',
  timed_out: 'Timed Out',
};

function formatElapsed(lastEventTime: string | null, lifecycle: AgentLifecycle): string | null {
  if (!lastEventTime) return null;
  const elapsed = Date.now() - new Date(lastEventTime).getTime();
  if (Number.isNaN(elapsed) || elapsed < 0) return null;
  if (lifecycle !== 'running') return null;
  if (elapsed < 1000) return '<1s';
  return `${Math.floor(elapsed / 1000)}s`;
}

function scoreColor(score: number): string {
  if (score >= 90) return 'var(--color-green-400)';
  if (score >= 70) return 'var(--color-warning-400)';
  return 'var(--color-danger-400)';
}

function mergeabilityHint(agent: AgentResult): { label: string; variant: 'success' | 'warning' | 'danger' } {
  if (agent.mergeable === true && agent.gateFailures.length === 0)
    return { label: 'MERGEABLE', variant: 'success' };
  if (agent.gateFailures.length > 0) return { label: 'GATED', variant: 'warning' };
  if (agent.mergeable === false) return { label: 'NOT MERGEABLE', variant: 'danger' };
  return { label: 'PENDING', variant: 'warning' };
}

export function LeaderboardRail({
  agents,
  raceResult,
  selectedAgent,
  onSelectAgent,
  raceError,
}: LeaderboardRailProps) {
  const resultMap = useMemo(() => {
    if (!raceResult) return new Map<string, AgentResult>();
    const m = new Map<string, AgentResult>();
    for (const a of raceResult.agents) m.set(a.agentKey, a);
    return m;
  }, [raceResult]);

  const sortedAgents = useMemo(() => {
    return [...agents].sort((a, b) => {
      const ra = resultMap.get(a.agentKey);
      const rb = resultMap.get(b.agentKey);
      if (ra && rb) return (rb.score ?? 0) - (ra.score ?? 0);
      if (ra) return -1;
      if (rb) return 1;
      return 0;
    });
  }, [agents, resultMap]);

  const containerStyle: CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    padding: 'var(--space-3)',
    gap: 'var(--space-2)',
    height: '100%',
  };

  const headerStyle: CSSProperties = {
    fontSize: 'var(--text-xs)',
    fontWeight: 'var(--weight-semibold)' as unknown as number,
    color: 'var(--color-text-muted)',
    textTransform: 'uppercase',
    letterSpacing: '0.05em',
    padding: '0 var(--space-2)',
    marginBottom: 'var(--space-1)',
  };

  return (
    <div style={containerStyle} data-testid="leaderboard-rail">
      <div style={headerStyle}>Leaderboard</div>

      {agents.length === 0 && (
        <div
          style={{
            color: 'var(--color-text-muted)',
            fontSize: 'var(--text-sm)',
            padding: 'var(--space-3) var(--space-2)',
            textAlign: 'center',
          }}
          data-testid="leaderboard-empty"
        >
          Start a race to see agents here
        </div>
      )}

      {sortedAgents.map((agent, idx) => (
        <LeaderboardCard
          key={agent.agentKey}
          agent={agent}
          result={resultMap.get(agent.agentKey) ?? null}
          rank={idx + 1}
          selected={selectedAgent === agent.agentKey}
          onSelect={() => onSelectAgent(agent.agentKey)}
        />
      ))}

      {raceError && (
        <div
          style={{
            fontSize: 'var(--text-xs)',
            color: 'var(--color-danger-400)',
            padding: 'var(--space-2)',
            borderRadius: 'var(--radius-sm)',
            backgroundColor: 'color-mix(in srgb, var(--color-danger-500) 8%, transparent)',
          }}
          data-testid="leaderboard-error"
        >
          {raceError}
        </div>
      )}
    </div>
  );
}

interface LeaderboardCardProps {
  agent: AgentStatus;
  result: AgentResult | null;
  rank: number;
  selected: boolean;
  onSelect: () => void;
}

function LeaderboardCard({ agent, result, rank, selected, onSelect }: LeaderboardCardProps) {
  const [hovered, setHovered] = useState(false);
  const isRunning = agent.lifecycle === 'running';
  const elapsed = formatElapsed(agent.lastEventTime, agent.lifecycle);
  const mergeHint = result ? mergeabilityHint(result) : null;
  const isFailed = agent.lifecycle === 'failed' || agent.lifecycle === 'timed_out';

  const cardStyle: CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    gap: 'var(--space-1)',
    padding: 'var(--space-2) var(--space-3)',
    borderRadius: 'var(--radius-md)',
    cursor: 'pointer',
    transition: 'all var(--transition-fast)',
    border: selected
      ? '1px solid var(--color-marine-500)'
      : '1px solid var(--color-border-700)',
    backgroundColor: selected
      ? 'color-mix(in srgb, var(--color-marine-500) 10%, var(--color-surface-800))'
      : hovered
        ? 'var(--color-surface-750)'
        : 'var(--color-surface-800)',
  };

  const topRowStyle: CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    gap: 'var(--space-2)',
  };

  const nameStyle: CSSProperties = {
    fontSize: 'var(--text-sm)',
    fontWeight: 'var(--weight-semibold)' as unknown as number,
    color: selected ? 'var(--color-text-primary)' : 'var(--color-text-secondary)',
    fontFamily: 'var(--font-mono)',
  };

  const rankStyle: CSSProperties = {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    width: 18,
    height: 18,
    borderRadius: 'var(--radius-full)',
    fontSize: '10px',
    fontWeight: 'var(--weight-bold)' as unknown as number,
    flexShrink: 0,
    backgroundColor: rank === 1 && result
      ? 'var(--color-green-500)'
      : 'var(--color-surface-700)',
    color: rank === 1 && result
      ? 'var(--color-text-inverse)'
      : 'var(--color-text-muted)',
  };

  const metaRowStyle: CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    gap: 'var(--space-2)',
    fontSize: 'var(--text-xs)',
    color: 'var(--color-text-muted)',
    flexWrap: 'wrap',
  };

  return (
    <button
      type="button"
      style={cardStyle}
      onClick={onSelect}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      data-testid={`leaderboard-card-${agent.agentKey}`}
    >
      <div style={topRowStyle}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)', minWidth: 0 }}>
          {result && <span style={rankStyle}>{rank}</span>}
          {isRunning && <PulsingDot />}
          <span style={nameStyle}>{agent.agentKey}</span>
        </div>
        <Badge variant={lifecycleBadgeVariant[agent.lifecycle]} dot>
          {lifecycleLabel[agent.lifecycle]}
        </Badge>
      </div>

      <div style={metaRowStyle}>
        {elapsed && <span>{elapsed} elapsed</span>}
        <span>{agent.eventCount} events</span>
        {result?.score != null && (
          <span style={{ color: scoreColor(result.score), fontFamily: 'var(--font-mono)', fontWeight: 'var(--weight-semibold)' as unknown as number }}>
            {Math.round(result.score)}/100
          </span>
        )}
        {result?.durationMs != null && (
          <span>{(result.durationMs / 1000).toFixed(1)}s</span>
        )}
      </div>

      {mergeHint && (
        <div style={{ marginTop: 'var(--space-1)' }}>
          <Badge variant={mergeHint.variant} dot>{mergeHint.label}</Badge>
        </div>
      )}

      {isFailed && (
        <div
          style={{
            fontSize: 'var(--text-xs)',
            color: 'var(--color-danger-400)',
            marginTop: 'var(--space-1)',
          }}
          data-testid={`leaderboard-failure-${agent.agentKey}`}
        >
          {agent.lifecycle === 'timed_out'
            ? 'Agent exceeded time limit'
            : 'Agent failed during execution'}
        </div>
      )}
    </button>
  );
}

function PulsingDot() {
  return (
    <>
      <style>{`@keyframes pulse-dot { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }`}</style>
      <span
        style={{
          width: 8,
          height: 8,
          borderRadius: '50%',
          backgroundColor: 'var(--color-marine-400)',
          flexShrink: 0,
          animation: 'pulse-dot 1.5s ease-in-out infinite',
        }}
      />
    </>
  );
}
