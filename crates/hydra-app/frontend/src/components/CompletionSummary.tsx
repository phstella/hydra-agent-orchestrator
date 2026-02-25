import { useMemo } from 'react';
import type { CSSProperties } from 'react';
import type { RaceResult, AgentResult } from '../types';
import { Badge, Button, Card } from './design-system';

interface CompletionSummaryProps {
  raceResult: RaceResult;
  selectedWinner: string | null;
  onSelectWinner: (agentKey: string) => void;
  onOpenReview: () => void;
}

function scoreColor(score: number): string {
  if (score >= 90) return 'var(--color-green-400)';
  if (score >= 70) return 'var(--color-warning-400)';
  return 'var(--color-danger-400)';
}

function formatDuration(ms: number | null): string {
  if (ms === null) return '—';
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

export function CompletionSummary({
  raceResult,
  selectedWinner,
  onSelectWinner,
  onOpenReview,
}: CompletionSummaryProps) {
  const topAgent = useMemo(() => {
    return [...raceResult.agents].sort((a, b) => (b.score ?? 0) - (a.score ?? 0))[0] ?? null;
  }, [raceResult.agents]);

  const winner = useMemo(() => {
    if (selectedWinner) {
      return raceResult.agents.find((a) => a.agentKey === selectedWinner) ?? topAgent;
    }
    return topAgent;
  }, [raceResult.agents, selectedWinner, topAgent]);

  const isMergeable = winner
    ? winner.mergeable === true && winner.gateFailures.length === 0
    : false;

  const containerStyle: CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    gap: 'var(--space-4)',
    padding: 'var(--space-6)',
  };

  const headerStyle: CSSProperties = {
    fontSize: 'var(--text-xl)',
    fontWeight: 'var(--weight-bold)' as unknown as number,
    color: 'var(--color-text-primary)',
    marginBottom: 'var(--space-2)',
  };

  const metaRowStyle: CSSProperties = {
    display: 'flex',
    gap: 'var(--space-3)',
    flexWrap: 'wrap',
    marginBottom: 'var(--space-4)',
  };

  return (
    <div style={containerStyle} data-testid="completion-summary">
      <div>
        <h2 style={headerStyle}>Race Complete</h2>
        <div style={metaRowStyle}>
          <Badge variant={raceResult.status === 'completed' ? 'success' : 'danger'} dot>
            {raceResult.status}
          </Badge>
          <Badge variant="neutral">
            {raceResult.agents.length} agent{raceResult.agents.length !== 1 ? 's' : ''}
          </Badge>
          {raceResult.durationMs !== null && (
            <Badge variant="neutral">
              {formatDuration(raceResult.durationMs)}
            </Badge>
          )}
          {raceResult.totalCost !== null && (
            <Badge variant="neutral">
              ${raceResult.totalCost.toFixed(2)}
            </Badge>
          )}
        </div>
      </div>

      {winner && (
        <Card variant={selectedWinner ? 'hero' : 'default'} padding="lg" data-testid="completion-winner-card">
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              marginBottom: 'var(--space-3)',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-3)' }}>
              <span
                style={{
                  fontSize: 'var(--text-2xl)',
                  fontWeight: 'var(--weight-bold)' as unknown as number,
                  fontFamily: 'var(--font-mono)',
                  color: 'var(--color-text-primary)',
                }}
              >
                {winner.agentKey}
              </span>
              {selectedWinner === winner.agentKey && (
                <Badge variant="success">WINNER</Badge>
              )}
            </div>
            {winner.score != null && (
              <span
                style={{
                  fontSize: 'var(--text-2xl)',
                  fontWeight: 'var(--weight-bold)' as unknown as number,
                  fontFamily: 'var(--font-mono)',
                  color: scoreColor(winner.score),
                }}
              >
                {Math.round(winner.score)}/100
              </span>
            )}
          </div>

          <div style={{ display: 'flex', gap: 'var(--space-2)', marginBottom: 'var(--space-3)', flexWrap: 'wrap' }}>
            <Badge variant={isMergeable ? 'success' : 'warning'} dot>
              {isMergeable ? 'MERGEABLE' : winner.gateFailures.length > 0 ? 'GATED' : 'NOT MERGEABLE'}
            </Badge>
            {winner.durationMs != null && (
              <Badge variant="neutral">{formatDuration(winner.durationMs)}</Badge>
            )}
          </div>

          {winner.gateFailures.length > 0 && (
            <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-danger-400)', marginBottom: 'var(--space-3)' }}>
              {winner.gateFailures.map((f, i) => (
                <div key={i}>• {f}</div>
              ))}
            </div>
          )}

          <div style={{ display: 'flex', gap: 'var(--space-3)', flexWrap: 'wrap' }}>
            {!selectedWinner && (
              <Button
                variant="primary"
                size="sm"
                onClick={() => onSelectWinner(winner.agentKey)}
                disabled={!isMergeable}
                data-testid="completion-select-winner"
              >
                Select as Winner
              </Button>
            )}
            <Button
              variant="secondary"
              size="sm"
              onClick={onOpenReview}
              data-testid="completion-open-review"
            >
              Open Diff Review
            </Button>
          </div>
        </Card>
      )}

      {raceResult.agents.length > 1 && (
        <OtherCandidates
          agents={raceResult.agents}
          winner={winner}
          selectedWinner={selectedWinner}
          onSelectWinner={onSelectWinner}
        />
      )}
    </div>
  );
}

function OtherCandidates({
  agents,
  winner,
  selectedWinner,
  onSelectWinner,
}: {
  agents: AgentResult[];
  winner: AgentResult | null;
  selectedWinner: string | null;
  onSelectWinner: (key: string) => void;
}) {
  const others = useMemo(() => {
    return [...agents]
      .filter((a) => a.agentKey !== winner?.agentKey)
      .sort((a, b) => (b.score ?? 0) - (a.score ?? 0));
  }, [agents, winner]);

  if (others.length === 0) return null;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2)' }}>
      <div
        style={{
          fontSize: 'var(--text-xs)',
          fontWeight: 'var(--weight-semibold)' as unknown as number,
          color: 'var(--color-text-muted)',
          textTransform: 'uppercase',
          letterSpacing: '0.05em',
        }}
      >
        Other Candidates
      </div>
      {others.map((agent) => {
        const mergeable = agent.mergeable === true && agent.gateFailures.length === 0;
        return (
          <Card key={agent.agentKey} padding="sm">
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
                <span
                  style={{
                    fontSize: 'var(--text-sm)',
                    fontFamily: 'var(--font-mono)',
                    color: 'var(--color-text-secondary)',
                  }}
                >
                  {agent.agentKey}
                </span>
                {agent.score != null && (
                  <span
                    style={{
                      fontSize: 'var(--text-sm)',
                      fontFamily: 'var(--font-mono)',
                      color: scoreColor(agent.score),
                    }}
                  >
                    {Math.round(agent.score)}
                  </span>
                )}
                <Badge variant={mergeable ? 'success' : 'warning'} dot>
                  {mergeable ? 'MERGEABLE' : 'GATED'}
                </Badge>
              </div>
              {selectedWinner !== agent.agentKey && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => onSelectWinner(agent.agentKey)}
                  disabled={!mergeable}
                >
                  Select
                </Button>
              )}
              {selectedWinner === agent.agentKey && (
                <Badge variant="success">WINNER</Badge>
              )}
            </div>
          </Card>
        );
      })}
    </div>
  );
}
