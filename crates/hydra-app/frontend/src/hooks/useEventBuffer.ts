import { useState, useCallback, useEffect, useRef } from 'react';
import type { AgentStreamEvent } from '../types';

const MAX_BUFFER_SIZE = 2000;
const FLUSH_INTERVAL_MS = 100;

interface UseEventBufferState {
  events: AgentStreamEvent[];
  push: (event: AgentStreamEvent) => void;
  clear: () => void;
  eventsByAgent: (agentKey: string) => AgentStreamEvent[];
}

/**
 * Bounded event buffer with backpressure.
 * Caps stored events at MAX_BUFFER_SIZE to prevent UI memory blowup.
 * Batches pushes via a flush interval so rapid streams don't cause
 * per-event re-renders.
 */
export function useEventBuffer(): UseEventBufferState {
  const [events, setEvents] = useState<AgentStreamEvent[]>([]);
  const pending = useRef<AgentStreamEvent[]>([]);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const flush = useCallback(() => {
    if (pending.current.length === 0) return;
    const batch = pending.current;
    pending.current = [];

    setEvents((prev) => {
      const merged = [...prev, ...batch];
      if (merged.length > MAX_BUFFER_SIZE) {
        return merged.slice(merged.length - MAX_BUFFER_SIZE);
      }
      return merged;
    });
  }, []);

  const push = useCallback(
    (event: AgentStreamEvent) => {
      pending.current.push(event);
      if (!timerRef.current) {
        timerRef.current = setTimeout(() => {
          timerRef.current = null;
          flush();
        }, FLUSH_INTERVAL_MS);
      }
    },
    [flush],
  );

  const clear = useCallback(() => {
    pending.current = [];
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    setEvents([]);
  }, []);

  const eventsByAgent = useCallback(
    (agentKey: string) => events.filter((e) => e.agentKey === agentKey),
    [events],
  );

  useEffect(
    () => () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    },
    [],
  );

  return { events, push, clear, eventsByAgent };
}
