import type { CSSProperties, ReactNode } from 'react';

type BadgeVariant = 'success' | 'warning' | 'danger' | 'info' | 'neutral' | 'experimental';

interface BadgeProps {
  variant?: BadgeVariant;
  children: ReactNode;
  dot?: boolean;
  style?: CSSProperties;
}

const variantColors: Record<BadgeVariant, { bg: string; text: string; dot: string }> = {
  success: {
    bg: 'rgba(34, 197, 94, 0.15)',
    text: 'var(--color-green-400)',
    dot: 'var(--color-green-500)',
  },
  warning: {
    bg: 'rgba(234, 179, 8, 0.15)',
    text: 'var(--color-warning-400)',
    dot: 'var(--color-warning-500)',
  },
  danger: {
    bg: 'rgba(239, 68, 68, 0.15)',
    text: 'var(--color-danger-400)',
    dot: 'var(--color-danger-500)',
  },
  info: {
    bg: 'rgba(47, 111, 159, 0.15)',
    text: 'var(--color-marine-400)',
    dot: 'var(--color-marine-500)',
  },
  neutral: {
    bg: 'rgba(107, 143, 128, 0.15)',
    text: 'var(--color-text-secondary)',
    dot: 'var(--color-text-muted)',
  },
  experimental: {
    bg: 'rgba(234, 179, 8, 0.15)',
    text: 'var(--color-warning-400)',
    dot: 'var(--color-warning-500)',
  },
};

export function Badge({ variant = 'neutral', children, dot = false, style }: BadgeProps) {
  const colors = variantColors[variant];

  const baseStyle: CSSProperties = {
    display: 'inline-flex',
    alignItems: 'center',
    gap: 'var(--space-1)',
    padding: '2px var(--space-2)',
    borderRadius: 'var(--radius-full)',
    fontSize: 'var(--text-xs)',
    fontWeight: 'var(--weight-medium)' as unknown as number,
    lineHeight: 'var(--leading-tight)',
    backgroundColor: colors.bg,
    color: colors.text,
    whiteSpace: 'nowrap',
    ...style,
  };

  const dotStyle: CSSProperties = {
    width: 6,
    height: 6,
    borderRadius: '50%',
    backgroundColor: colors.dot,
    flexShrink: 0,
  };

  return (
    <span style={baseStyle}>
      {dot && <span style={dotStyle} />}
      {children}
    </span>
  );
}
