import type { ButtonHTMLAttributes, CSSProperties, ReactNode } from 'react';

type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger';
type ButtonSize = 'sm' | 'md' | 'lg';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  loading?: boolean;
  icon?: ReactNode;
}

const variantStyles: Record<ButtonVariant, CSSProperties> = {
  primary: {
    background: 'var(--color-green-500)',
    color: 'var(--color-text-inverse)',
    border: 'none',
  },
  secondary: {
    background: 'transparent',
    color: 'var(--color-text-primary)',
    border: '1px solid var(--color-border-700)',
  },
  ghost: {
    background: 'transparent',
    color: 'var(--color-text-secondary)',
    border: '1px solid transparent',
  },
  danger: {
    background: 'var(--color-danger-500)',
    color: 'var(--color-text-primary)',
    border: 'none',
  },
};

const sizeStyles: Record<ButtonSize, CSSProperties> = {
  sm: { padding: 'var(--space-1) var(--space-3)', fontSize: 'var(--text-sm)' },
  md: { padding: 'var(--space-2) var(--space-4)', fontSize: 'var(--text-sm)' },
  lg: { padding: 'var(--space-3) var(--space-6)', fontSize: 'var(--text-base)' },
};

export function Button({
  variant = 'primary',
  size = 'md',
  loading = false,
  icon,
  children,
  disabled,
  style,
  ...props
}: ButtonProps) {
  const isDisabled = disabled || loading;

  const baseStyle: CSSProperties = {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 'var(--space-2)',
    borderRadius: 'var(--radius-md)',
    fontWeight: 'var(--weight-medium)' as unknown as number,
    fontFamily: 'var(--font-family)',
    cursor: isDisabled ? 'not-allowed' : 'pointer',
    opacity: isDisabled ? 0.5 : 1,
    transition: 'all var(--transition-fast)',
    lineHeight: 'var(--leading-tight)',
    whiteSpace: 'nowrap',
    ...variantStyles[variant],
    ...sizeStyles[size],
    ...style,
  };

  return (
    <button style={baseStyle} disabled={isDisabled} {...props}>
      {loading && <span style={{ display: 'inline-block', width: '1em', height: '1em', border: '2px solid currentColor', borderTopColor: 'transparent', borderRadius: '50%', animation: 'spin 0.6s linear infinite' }} />}
      {!loading && icon}
      {children}
    </button>
  );
}
