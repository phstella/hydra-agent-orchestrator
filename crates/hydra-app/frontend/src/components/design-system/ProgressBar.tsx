import type { CSSProperties } from 'react';

interface ProgressBarProps {
  value: number;
  max?: number;
  variant?: 'green' | 'marine' | 'warning' | 'gradient';
  height?: number;
  showLabel?: boolean;
  style?: CSSProperties;
}

export function ProgressBar({
  value,
  max = 100,
  variant = 'green',
  height = 8,
  showLabel = false,
  style,
}: ProgressBarProps) {
  const pct = Math.min(100, Math.max(0, (value / max) * 100));

  const trackStyle: CSSProperties = {
    width: '100%',
    height,
    backgroundColor: 'var(--color-bg-900)',
    borderRadius: 'var(--radius-full)',
    overflow: 'hidden',
    position: 'relative',
    ...style,
  };

  const fills: Record<string, string> = {
    green: 'var(--color-green-500)',
    marine: 'var(--color-marine-500)',
    warning: 'var(--color-warning-500)',
    gradient: `linear-gradient(90deg, var(--color-green-500) 0%, var(--color-warning-500) 60%, var(--color-danger-500) 100%)`,
  };

  const fillStyle: CSSProperties = {
    height: '100%',
    width: `${pct}%`,
    borderRadius: 'var(--radius-full)',
    transition: 'width var(--transition-base)',
    ...(variant === 'gradient'
      ? { backgroundImage: fills[variant] }
      : { backgroundColor: fills[variant] }),
  };

  const wrapperStyle: CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    gap: 'var(--space-2)',
  };

  const labelStyle: CSSProperties = {
    fontSize: 'var(--text-xs)',
    color: 'var(--color-text-muted)',
    minWidth: 36,
    textAlign: 'right',
  };

  if (showLabel) {
    return (
      <div style={wrapperStyle}>
        <div style={trackStyle}>
          <div style={fillStyle} />
        </div>
        <span style={labelStyle}>{Math.round(pct)}%</span>
      </div>
    );
  }

  return (
    <div style={trackStyle}>
      <div style={fillStyle} />
    </div>
  );
}
