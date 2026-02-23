import type { CSSProperties, ReactNode } from 'react';

interface CardProps {
  children: ReactNode;
  variant?: 'default' | 'elevated' | 'outlined' | 'hero';
  padding?: 'none' | 'sm' | 'md' | 'lg';
  style?: CSSProperties;
}

const paddings: Record<string, string> = {
  none: '0',
  sm: 'var(--space-3)',
  md: 'var(--space-4)',
  lg: 'var(--space-6)',
};

export function Card({
  children,
  variant = 'default',
  padding = 'md',
  style,
}: CardProps) {
  const baseStyle: CSSProperties = {
    borderRadius: 'var(--radius-lg)',
    padding: paddings[padding],
    transition: 'all var(--transition-fast)',
  };

  const variants: Record<string, CSSProperties> = {
    default: {
      backgroundColor: 'var(--color-surface-800)',
      border: '1px solid var(--color-border-700)',
    },
    elevated: {
      backgroundColor: 'var(--color-surface-800)',
      border: '1px solid var(--color-border-700)',
      boxShadow: 'var(--shadow-md)',
    },
    outlined: {
      backgroundColor: 'transparent',
      border: '1px solid var(--color-border-700)',
    },
    hero: {
      backgroundColor: 'var(--color-surface-800)',
      border: '1px solid var(--color-green-500)',
      boxShadow: 'var(--shadow-glow-green)',
    },
  };

  return (
    <div style={{ ...baseStyle, ...variants[variant], ...style }}>
      {children}
    </div>
  );
}
