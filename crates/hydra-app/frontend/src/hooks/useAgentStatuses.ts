import { useMemo } from 'react';
import type { AgentStreamEvent } from '../types';

export type AgentLifecycle = 'running' | 'completed' | 'failed' | 'timed_out';

export interface AgentStatus {
  agentKey: string;
  lifecycle: AgentLifecycle;
  eventCount: number;
  lastEventTime: string | null;
}

const TERMINAL_EVENT_MAP: Record<string, AgentLifecycle> = {
  agent_completed: 'completed',
  agent_failed: 'failed',
  agent_timed_out: 'timed_out',
  agent_timeout: 'timed_out',
};

/**
 * Derives per-agent lifecycle status from the event stream.
 * An agent is "running" until a terminal event is seen.
 */
export function useAgentStatuses(
  events: AgentStreamEvent[],
  knownAgents: string[],
): AgentStatus[] {
  return useMemo(() => {
    const statusMap = new Map<string, AgentStatus>();

    for (const key of knownAgents) {
      statusMap.set(key, {
        agentKey: key,
        lifecycle: 'running',
        eventCount: 0,
        lastEventTime: null,
      });
    }

    for (const evt of events) {
      if (evt.agentKey === 'system') continue;

      let entry = statusMap.get(evt.agentKey);
      if (!entry) {
        entry = {
          agentKey: evt.agentKey,
          lifecycle: 'running',
          eventCount: 0,
          lastEventTime: null,
        };
        statusMap.set(evt.agentKey, entry);
      }

      entry.eventCount += 1;
      entry.lastEventTime = evt.timestamp;

      const terminal = TERMINAL_EVENT_MAP[evt.eventType];
      if (terminal) {
        entry.lifecycle = terminal;
      }
    }

    return Array.from(statusMap.values());
  }, [events, knownAgents]);
}
