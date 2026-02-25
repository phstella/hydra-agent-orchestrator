import { useCallback, useMemo } from 'react';
import type { CSSProperties } from 'react';
import type { RaceResult, AgentResult, DimensionScore } from '../types';
import { Card, Badge, Button, Panel, ProgressBar } from './design-system';

interface ResultsScoreboardProps {
  result: RaceResult;
  selectedWinner: string | null;
  onSelectWinner: (agentKey: string) => void;
}

const DIMENSION_LABELS: Record<string, string> = {
  build: 'Build Status',
  tests: 'Unit Tests',
  lint: 'Lint Issues',
  diff_scope: 'Diff Scope',
  speed: 'Generation Speed',
};

const DIMENSION_ORDER = ['build', 'tests', 'lint', 'diff_scope', 'speed'];

function mergeabilityVariant(agent: AgentResult): 'success' | 'warning' | 'danger' {
  if (agent.mergeable === true && agent.gateFailures.length === 0) return 'success';
  if (agent.mergeable === false) return 'danger';
  return 'warning';
}

function mergeabilityLabel(agent: AgentResult): string {
  if (agent.mergeable === true && agent.gateFailures.length === 0) return 'MERGEABLE';
  if (agent.gateFailures.length > 0) return 'GATED';
  if (agent.mergeable === false) return 'NOT MERGEABLE';
  return 'WARNINGS';
}

function formatDuration(ms: number | null): string {
  if (ms === null) return '—';
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

function formatCost(cost: number | null): string {
  if (cost === null) return '—';
  return `$${cost.toFixed(2)}`;
}

function formatDimensionEvidence(dim: DimensionScore): string {
  const ev = dim.evidence as Record<string, unknown> | null;
  if (!ev) return `${Math.round(dim.score)}`;

  switch (dim.name) {
    case 'build':
      return dim.score >= 100 ? 'PASS' : 'FAIL';
    case 'tests': {
      const passed = ev.passed as number | undefined;
      const total = ((ev.passed as number | undefined) ?? 0) + ((ev.failed as number | undefined) ?? 0);
      if (passed !== undefined) return `${passed}/${total || passed}`;
      return `${Math.round(dim.score)}`;
    }
    case 'lint': {
      const current = ev.current_warnings as number | undefined;
      if (current !== undefined) return `${current}`;
      return `${Math.round(dim.score)}`;
    }
    case 'diff_scope': {
      const added = ev.lines_added as number | undefined;
      const removed = ev.lines_removed as number | undefined;
      if (added !== undefined && removed !== undefined) return `+${added} / -${removed}`;
      return `${Math.round(dim.score)}`;
    }
    case 'speed': {
      const agentMs = ev.agent_duration_ms as number | undefined;
      if (agentMs !== undefined) return formatDuration(agentMs);
      return `${Math.round(dim.score)}`;
    }
    default:
      return `${Math.round(dim.score)}`;
  }
}

function scoreColor(score: number): string {
  if (score >= 90) return 'var(--color-green-400)';
  if (score >= 70) return 'var(--color-warning-400)';
  return 'var(--color-danger-400)';
}

function progressVariant(score: number): 'green' | 'warning' | 'gradient' {
  if (score >= 90) return 'green';
  if (score >= 70) return 'warning';
  return 'gradient';
}

function qualityCoverageWarning(agents: AgentResult[]): string | null {
  const qualityDims = new Set(['build', 'tests', 'lint']);
  const presentQuality = new Set<string>();
  for (const agent of agents) {
    for (const dim of agent.dimensions) {
      if (qualityDims.has(dim.name)) {
        presentQuality.add(dim.name);
      }
    }
  }

  if (presentQuality.size > 0) return null;
  return 'Quality checks are not configured for this workspace; ranking is based on diff scope and speed only.';
}

export function ResultsScoreboard({ result, selectedWinner, onSelectWinner }: ResultsScoreboardProps) {
  const sortedAgents = useMemo(
    () =>
      [...result.agents].sort((a, b) => (b.score ?? 0) - (a.score ?? 0)),
    [result.agents],
  );
  const qualityWarning = useMemo(() => qualityCoverageWarning(sortedAgents), [sortedAgents]);

  const allDimensionNames = useMemo(() => {
    const seen = new Set<string>();
    for (const agent of sortedAgents) {
      for (const dim of agent.dimensions) {
        seen.add(dim.name);
      }
    }
    return DIMENSION_ORDER.filter((d) => seen.has(d)).concat(
      [...seen].filter((d) => !DIMENSION_ORDER.includes(d)),
    );
  }, [sortedAgents]);

  const handleSelectWinner = useCallback(
    (agentKey: string) => {
      onSelectWinner(agentKey);
    },
    [onSelectWinner],
  );

  const containerStyle: CSSProperties = {
    maxWidth: 1080,
    margin: '0 auto',
    padding: 'var(--space-8) var(--space-6)',
  };

  const headerStyle: CSSProperties = {
    marginBottom: 'var(--space-6)',
  };

  const titleStyle: CSSProperties = {
    fontSize: 'var(--text-2xl)',
    fontWeight: 'var(--weight-bold)' as unknown as number,
    color: 'var(--color-text-primary)',
    marginBottom: 'var(--space-2)',
  };

  const subtitleStyle: CSSProperties = {
    fontSize: 'var(--text-sm)',
    color: 'var(--color-text-secondary)',
    marginBottom: 'var(--space-4)',
  };

  const metaRowStyle: CSSProperties = {
    display: 'flex',
    gap: 'var(--space-3)',
    flexWrap: 'wrap',
  };

  const cardsRowStyle: CSSProperties = {
    display: 'grid',
    gridTemplateColumns: `repeat(${Math.min(sortedAgents.length, 4)}, 1fr)`,
    gap: 'var(--space-4)',
    marginBottom: 'var(--space-8)',
  };

  return (
    <div style={containerStyle}>
      <div style={headerStyle}>
        <h1 style={titleStyle}>Orchestration Results</h1>
        <div style={subtitleStyle}>
          Comparing {result.agents.length} agent candidate{result.agents.length !== 1 ? 's' : ''} based on composite scoring
        </div>
        <div style={metaRowStyle}>
          <Badge variant="neutral">Run {result.runId.slice(0, 8)}</Badge>
          <Badge variant={result.status === 'completed' ? 'success' : result.status === 'failed' ? 'danger' : 'info'}>
            {result.status}
          </Badge>
          {result.durationMs !== null && (
            <Badge variant="neutral">Duration {formatDuration(result.durationMs)}</Badge>
          )}
          {result.totalCost !== null && (
            <Badge variant="neutral">Cost {formatCost(result.totalCost)}</Badge>
          )}
        </div>
        {qualityWarning && (
          <div
            style={{
              marginTop: 'var(--space-3)',
              padding: 'var(--space-2) var(--space-3)',
              borderRadius: 'var(--radius-md)',
              border: '1px solid var(--color-warning-500)',
              backgroundColor: 'color-mix(in srgb, var(--color-warning-500) 10%, transparent)',
              color: 'var(--color-warning-400)',
              fontSize: 'var(--text-xs)',
            }}
          >
            {qualityWarning} Add `scoring.profile` or `scoring.commands` in `hydra.toml` for fuller evaluation.
          </div>
        )}
      </div>

      {/* Ranked candidate cards */}
      <div style={cardsRowStyle}>
        {sortedAgents.map((agent, rank) => (
          <CandidateCard
            key={agent.agentKey}
            agent={agent}
            rank={rank + 1}
            isWinner={selectedWinner === agent.agentKey}
            onSelect={handleSelectWinner}
          />
        ))}
      </div>

      {/* Score breakdown table */}
      <Panel title="Score Breakdown">
        <ScoreBreakdownTable
          agents={sortedAgents}
          dimensionNames={allDimensionNames}
          winner={selectedWinner}
        />
      </Panel>
    </div>
  );
}

// ---------------------------------------------------------------------------
// CandidateCard
// ---------------------------------------------------------------------------

interface CandidateCardProps {
  agent: AgentResult;
  rank: number;
  isWinner: boolean;
  onSelect: (agentKey: string) => void;
}

function CandidateCard({ agent, rank, isWinner, onSelect }: CandidateCardProps) {
  const isMergeable = agent.mergeable === true && agent.gateFailures.length === 0;
  const canSelect = isMergeable;

  const cardStyle: CSSProperties = {
    position: 'relative',
    ...(isWinner
      ? {
          border: '1px solid var(--color-green-500)',
          boxShadow: 'var(--shadow-glow-green)',
        }
      : {}),
  };

  const rankBadgeStyle: CSSProperties = {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    width: 22,
    height: 22,
    borderRadius: 'var(--radius-full)',
    fontSize: 'var(--text-xs)',
    fontWeight: 'var(--weight-bold)' as unknown as number,
    backgroundColor: rank === 1 ? 'var(--color-green-500)' : 'var(--color-surface-700)',
    color: rank === 1 ? 'var(--color-text-inverse)' : 'var(--color-text-secondary)',
    flexShrink: 0,
  };

  const agentNameStyle: CSSProperties = {
    fontSize: 'var(--text-sm)',
    fontWeight: 'var(--weight-semibold)' as unknown as number,
    color: 'var(--color-text-primary)',
  };

  const scoreStyle: CSSProperties = {
    fontSize: 'var(--text-3xl)',
    fontWeight: 'var(--weight-bold)' as unknown as number,
    color: scoreColor(agent.score ?? 0),
    lineHeight: 'var(--leading-tight)',
    fontFamily: 'var(--font-mono)',
  };

  const subscriptStyle: CSSProperties = {
    fontSize: 'var(--text-sm)',
    color: 'var(--color-text-muted)',
    fontWeight: 'var(--weight-normal)' as unknown as number,
    fontFamily: 'var(--font-family)',
  };

  const gateListStyle: CSSProperties = {
    fontSize: 'var(--text-xs)',
    color: 'var(--color-danger-400)',
    marginTop: 'var(--space-1)',
  };

  return (
    <Card variant={isWinner ? 'hero' : 'default'} padding="lg" style={cardStyle}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)', marginBottom: 'var(--space-3)' }}>
        <span style={rankBadgeStyle}>{rank}</span>
        <span style={agentNameStyle}>{agent.agentKey}</span>
        {isWinner && (
          <Badge variant="success" style={{ marginLeft: 'auto' }}>WINNER</Badge>
        )}
      </div>

      <div style={{ marginBottom: 'var(--space-3)' }}>
        <span style={scoreStyle}>{agent.score !== null ? Math.round(agent.score) : '—'}</span>
        <span style={subscriptStyle}> /100</span>
      </div>

      <div style={{ marginBottom: 'var(--space-3)' }}>
        <Badge variant={mergeabilityVariant(agent)} dot>
          {mergeabilityLabel(agent)}
        </Badge>
        {agent.gateFailures.length > 0 && (
          <div style={gateListStyle}>
            {agent.gateFailures.map((f, i) => (
              <div key={i}>• {f}</div>
            ))}
          </div>
        )}
      </div>

      {agent.durationMs !== null && (
        <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)', marginBottom: 'var(--space-3)' }}>
          Duration: {formatDuration(agent.durationMs)}
        </div>
      )}

      <Button
        variant={isWinner ? 'primary' : 'secondary'}
        size="sm"
        onClick={() => onSelect(agent.agentKey)}
        disabled={!canSelect}
        style={{ width: '100%' }}
      >
        {isWinner ? '✓ Selected as Winner' : 'Select as Winner'}
      </Button>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// ScoreBreakdownTable
// ---------------------------------------------------------------------------

interface ScoreBreakdownTableProps {
  agents: AgentResult[];
  dimensionNames: string[];
  winner: string | null;
}

function ScoreBreakdownTable({ agents, dimensionNames, winner }: ScoreBreakdownTableProps) {
  const thStyle: CSSProperties = {
    padding: 'var(--space-3) var(--space-4)',
    textAlign: 'left',
    fontSize: 'var(--text-xs)',
    fontWeight: 'var(--weight-semibold)' as unknown as number,
    color: 'var(--color-text-muted)',
    textTransform: 'uppercase',
    letterSpacing: '0.05em',
    borderBottom: '1px solid var(--color-border-700)',
    whiteSpace: 'nowrap',
  };

  const tdStyle: CSSProperties = {
    padding: 'var(--space-3) var(--space-4)',
    borderBottom: '1px solid var(--color-border-700)',
    fontSize: 'var(--text-sm)',
    verticalAlign: 'middle',
  };

  const dimLabelStyle: CSSProperties = {
    ...tdStyle,
    color: 'var(--color-text-secondary)',
    fontWeight: 'var(--weight-medium)' as unknown as number,
    whiteSpace: 'nowrap',
  };

  function getDimension(agent: AgentResult, name: string): DimensionScore | undefined {
    return agent.dimensions.find((d) => d.name === name);
  }

  return (
    <div style={{ overflowX: 'auto' }}>
      <table style={{ width: '100%', borderCollapse: 'collapse' }}>
        <thead>
          <tr>
            <th style={{ ...thStyle, minWidth: 140 }}>Dimension</th>
            {agents.map((agent) => (
              <th key={agent.agentKey} style={{ ...thStyle, textAlign: 'center', minWidth: 120 }}>
                <div>{agent.agentKey}</div>
                {winner === agent.agentKey && (
                  <Badge variant="success" style={{ marginTop: 2 }}>WINNER</Badge>
                )}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {dimensionNames.map((name) => (
            <tr key={name}>
              <td style={dimLabelStyle}>
                {DIMENSION_LABELS[name] ?? name}
              </td>
              {agents.map((agent) => {
                const dim = getDimension(agent, name);
                if (!dim) {
                  return (
                    <td key={agent.agentKey} style={{ ...tdStyle, textAlign: 'center', color: 'var(--color-text-muted)' }}>
                      —
                    </td>
                  );
                }

                const evidenceText = formatDimensionEvidence(dim);
                const isBuildDim = name === 'build';
                const buildPassed = isBuildDim && dim.score >= 100;
                const buildFailed = isBuildDim && dim.score < 100;

                return (
                  <td key={agent.agentKey} style={{ ...tdStyle, textAlign: 'center' }}>
                    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 'var(--space-1)' }}>
                      {isBuildDim ? (
                        <Badge variant={buildPassed ? 'success' : 'danger'}>
                          {buildFailed ? 'FAIL' : 'PASS'}
                        </Badge>
                      ) : (
                        <>
                          <span style={{ color: scoreColor(dim.score), fontFamily: 'var(--font-mono)', fontWeight: 'var(--weight-semibold)' as unknown as number }}>
                            {evidenceText}
                          </span>
                          <ProgressBar value={dim.score} variant={progressVariant(dim.score)} height={4} style={{ width: '100%', maxWidth: 80 }} />
                        </>
                      )}
                    </div>
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
