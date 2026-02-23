import { useEffect } from 'react';
import { Card, Badge, Button, Panel, ProgressBar } from './design-system';
import { usePreflight } from '../hooks';
import type { DiagnosticCheck, AdapterInfo, CheckStatus } from '../types';
import { isAdapterAvailable, isExperimental, statusToVariant } from '../types';

function CheckIcon({ status }: { status: CheckStatus }) {
  const color =
    status === 'passed'
      ? 'var(--color-green-500)'
      : status === 'failed'
        ? 'var(--color-danger-500)'
        : status === 'warning'
          ? 'var(--color-warning-500)'
          : 'var(--color-marine-500)';

  const symbol = status === 'passed' ? '✓' : status === 'failed' ? '✗' : status === 'warning' ? '!' : '⟳';

  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        width: 28,
        height: 28,
        borderRadius: '50%',
        backgroundColor: `color-mix(in srgb, ${color} 15%, transparent)`,
        color,
        fontSize: 'var(--text-sm)',
        fontWeight: 'var(--weight-bold)' as unknown as number,
        flexShrink: 0,
      }}
    >
      {symbol}
    </span>
  );
}

function DiagnosticRow({ check }: { check: DiagnosticCheck }) {
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 'var(--space-3)',
        padding: 'var(--space-3) var(--space-4)',
        borderBottom: '1px solid var(--color-border-700)',
      }}
    >
      <CheckIcon status={check.status} />
      <div style={{ flex: 1 }}>
        <div style={{ fontSize: 'var(--text-sm)', fontWeight: 'var(--weight-medium)' as unknown as number, color: 'var(--color-text-primary)' }}>
          {check.name}
        </div>
        <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)', marginTop: 2 }}>
          {check.description}
        </div>
      </div>
      <Badge variant={statusToVariant(check.status)}>
        {check.status.toUpperCase()}
      </Badge>
    </div>
  );
}

function AdapterBadge({ adapter }: { adapter: AdapterInfo }) {
  const available = isAdapterAvailable(adapter.status);
  const experimental = isExperimental(adapter);

  return (
    <Badge
      variant={experimental ? 'experimental' : available ? 'success' : 'danger'}
      dot
    >
      {adapter.key}
      {adapter.version ? ` ${adapter.version}` : ''}
    </Badge>
  );
}

export function PreflightDashboard() {
  const { result, loading, error, refresh } = usePreflight();

  useEffect(() => {
    refresh();
  }, [refresh]);

  if (error) {
    return (
      <div style={{ padding: 'var(--space-8)', textAlign: 'center' }}>
        <div style={{ color: 'var(--color-danger-400)', marginBottom: 'var(--space-4)' }}>
          Failed to load diagnostics: {error}
        </div>
        <Button variant="secondary" onClick={refresh}>
          Retry
        </Button>
      </div>
    );
  }

  if (!result && loading) {
    return (
      <div style={{ padding: 'var(--space-8)', textAlign: 'center', color: 'var(--color-text-muted)' }}>
        Running diagnostics...
      </div>
    );
  }

  if (!result) return null;

  const readyLabel = result.systemReady ? 'System Ready' : 'System Not Ready';
  const readyColor = result.systemReady ? 'var(--color-green-500)' : 'var(--color-danger-500)';

  return (
    <div style={{ maxWidth: 960, margin: '0 auto', padding: 'var(--space-8) var(--space-6)' }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', marginBottom: 'var(--space-8)' }}>
        <div>
          <h1 style={{ fontSize: 'var(--text-2xl)', fontWeight: 'var(--weight-bold)' as unknown as number, marginBottom: 'var(--space-2)' }}>
            System Pre-flight & Diagnostics
          </h1>
          <p style={{ color: 'var(--color-text-muted)', fontSize: 'var(--text-sm)', maxWidth: 480 }}>
            Verifying local environment integrity, adapter connections, and running startup smoke tests for parallel agent orchestration.
          </p>
        </div>
        <div style={{ display: 'flex', gap: 'var(--space-3)' }}>
          <Button variant="secondary" size="sm">
            View Logs
          </Button>
          <Button variant="secondary" size="sm" onClick={refresh} loading={loading}>
            Re-run Diagnostics
          </Button>
        </div>
      </div>

      {/* Hero readiness card */}
      <Card variant="hero" padding="lg" style={{ marginBottom: 'var(--space-6)' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-4)' }}>
            <span
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                width: 48,
                height: 48,
                borderRadius: '50%',
                backgroundColor: result.systemReady ? 'rgba(34, 197, 94, 0.15)' : 'rgba(239, 68, 68, 0.15)',
                color: readyColor,
                fontSize: 'var(--text-xl)',
              }}
            >
              {result.systemReady ? '✓' : '✗'}
            </span>
            <div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-3)' }}>
                <span style={{ fontSize: 'var(--text-xl)', fontWeight: 'var(--weight-bold)' as unknown as number }}>
                  {readyLabel}
                </span>
                <Badge variant={result.systemReady ? 'success' : 'danger'}>
                  {result.passedCount}/{result.totalCount} Passed
                </Badge>
              </div>
              <p style={{ color: 'var(--color-text-muted)', fontSize: 'var(--text-sm)', marginTop: 'var(--space-1)' }}>
                {result.systemReady
                  ? 'All subsystems operational. You are ready to merge.'
                  : `${result.failedCount} check(s) failed. Review diagnostics below.`}
              </p>
            </div>
          </div>
          <div style={{ textAlign: 'right' }}>
            <div style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-muted)', marginBottom: 'var(--space-2)' }}>
              Health Score
            </div>
            <div style={{ fontSize: 'var(--text-2xl)', fontWeight: 'var(--weight-bold)' as unknown as number, color: readyColor, marginBottom: 'var(--space-2)' }}>
              {Math.round(result.healthScore)}%
            </div>
            <ProgressBar value={result.healthScore} variant="green" height={6} />
          </div>
        </div>
      </Card>

      {/* Two-column layout: Diagnostics + Environment */}
      <div style={{ display: 'grid', gridTemplateColumns: '1.4fr 1fr', gap: 'var(--space-6)' }}>
        {/* Diagnostics checks */}
        <div>
          <h2 style={{ fontSize: 'var(--text-lg)', fontWeight: 'var(--weight-semibold)' as unknown as number, marginBottom: 'var(--space-4)' }}>
            Diagnostic Checks
          </h2>
          <Card padding="none">
            {result.checks.map((check, i) => (
              <DiagnosticRow key={i} check={check} />
            ))}
          </Card>
        </div>

        {/* Environment panel */}
        <div>
          <h2 style={{ fontSize: 'var(--text-lg)', fontWeight: 'var(--weight-semibold)' as unknown as number, marginBottom: 'var(--space-4)' }}>
            Environment
          </h2>

          <Panel title="Active Adapters" style={{ marginBottom: 'var(--space-4)' }}>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 'var(--space-2)' }}>
              {result.adapters.map((adapter) => (
                <AdapterBadge key={adapter.key} adapter={adapter} />
              ))}
            </div>
          </Panel>

          {result.warnings.length > 0 && (
            <Card
              variant="outlined"
              style={{
                borderColor: 'var(--color-warning-500)',
                backgroundColor: 'rgba(234, 179, 8, 0.05)',
              }}
            >
              {result.warnings.map((w, i) => (
                <div
                  key={i}
                  style={{
                    display: 'flex',
                    alignItems: 'flex-start',
                    gap: 'var(--space-2)',
                    fontSize: 'var(--text-sm)',
                    color: 'var(--color-warning-400)',
                    marginBottom: i < result.warnings.length - 1 ? 'var(--space-2)' : 0,
                  }}
                >
                  <span>⚠</span>
                  <span>{w}</span>
                </div>
              ))}
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}
