import { useState, useCallback, useRef } from 'react';
import type { PreflightResult } from '../types';
import { runPreflight } from '../ipc';

interface UsePreflightState {
  result: PreflightResult | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

export function usePreflight(): UsePreflightState {
  const [result, setResult] = useState<PreflightResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inflight = useRef(false);

  const refresh = useCallback(async () => {
    if (inflight.current) return;
    inflight.current = true;
    setLoading(true);
    setError(null);

    try {
      const data = await runPreflight();
      setResult(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
      inflight.current = false;
    }
  }, []);

  return { result, loading, error, refresh };
}
