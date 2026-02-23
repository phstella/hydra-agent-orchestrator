import { useState } from 'react';
import type { CSSProperties } from 'react';
import { Badge } from './design-system';
import type { AgentLifecycle, AgentStatus } from '../hooks/useAgentStatuses';

interface AgentRailProps {
  agents: AgentStatus[];
  selectedAgent: string | null;
  onSelectAgent: (agentKey: string) => void;
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

export function AgentRail({ agents, selectedAgent, onSelectAgent }: AgentRailProps) {
  const containerStyle: CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    gap: 'var(--space-2)',
    minWidth: 220,
  };

  return (
    <div style={containerStyle}>
      <div
        style={{
          fontSize: 'var(--text-xs)',
          fontWeight: 'var(--weight-semibold)' as unknown as number,
          color: 'var(--color-text-muted)',
          textTransform: 'uppercase',
          letterSpacing: '0.05em',
          padding: '0 var(--space-2)',
          marginBottom: 'var(--space-1)',
        }}
      >
        Agents
      </div>
      {agents.map((agent) => (
        <AgentRailItem
          key={agent.agentKey}
          agent={agent}
          selected={selectedAgent === agent.agentKey}
          onSelect={() => onSelectAgent(agent.agentKey)}
        />
      ))}
      {agents.length === 0 && (
        <div
          style={{
            color: 'var(--color-text-muted)',
            fontSize: 'var(--text-sm)',
            padding: 'var(--space-3) var(--space-2)',
          }}
        >
          No agents active
        </div>
      )}
    </div>
  );
}

function AgentRailItem({
  agent,
  selected,
  onSelect,
}: {
  agent: AgentStatus;
  selected: boolean;
  onSelect: () => void;
}) {
  const [hovered, setHovered] = useState(false);

  const isRunning = agent.lifecycle === 'running';

  const itemStyle: CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    gap: 'var(--space-2)',
    padding: 'var(--space-2) var(--space-3)',
    borderRadius: 'var(--radius-md)',
    cursor: 'pointer',
    transition: 'all var(--transition-fast)',
    border: selected
      ? '1px solid var(--color-marine-500)'
      : '1px solid transparent',
    backgroundColor: selected
      ? 'color-mix(in srgb, var(--color-marine-500) 10%, var(--color-surface-800))'
      : hovered
        ? 'var(--color-surface-750)'
        : 'transparent',
  };

  const nameStyle: CSSProperties = {
    fontSize: 'var(--text-sm)',
    fontWeight: selected
      ? ('var(--weight-semibold)' as unknown as number)
      : ('var(--weight-normal)' as unknown as number),
    color: selected ? 'var(--color-text-primary)' : 'var(--color-text-secondary)',
    fontFamily: 'var(--font-mono)',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  };

  const metaStyle: CSSProperties = {
    fontSize: 'var(--text-xs)',
    color: 'var(--color-text-muted)',
    marginTop: 2,
  };

  return (
    <button
      type="button"
      style={itemStyle}
      onClick={onSelect}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <div style={{ minWidth: 0, flex: 1 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
          {isRunning && <PulsingDot />}
          <span style={nameStyle}>{agent.agentKey}</span>
        </div>
        <div style={metaStyle}>{agent.eventCount} events</div>
      </div>
      <Badge variant={lifecycleBadgeVariant[agent.lifecycle]} dot>
        {lifecycleLabel[agent.lifecycle]}
      </Badge>
    </button>
  );
}

function PulsingDot() {
  const dotStyle: CSSProperties = {
    width: 8,
    height: 8,
    borderRadius: '50%',
    backgroundColor: 'var(--color-marine-400)',
    flexShrink: 0,
    animation: 'pulse-dot 1.5s ease-in-out infinite',
  };

  return (
    <>
      <style>{`
        @keyframes pulse-dot {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.3; }
        }
      `}</style>
      <span style={dotStyle} />
    </>
  );
}
