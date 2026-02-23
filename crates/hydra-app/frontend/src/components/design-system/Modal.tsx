import { useEffect, useCallback } from 'react';
import type { CSSProperties, ReactNode } from 'react';

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: ReactNode;
  footer?: ReactNode;
  width?: string;
}

export function Modal({ open, onClose, title, children, footer, width = '480px' }: ModalProps) {
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    },
    [onClose],
  );

  useEffect(() => {
    if (open) {
      document.addEventListener('keydown', handleKeyDown);
      return () => document.removeEventListener('keydown', handleKeyDown);
    }
  }, [open, handleKeyDown]);

  if (!open) return null;

  const backdropStyle: CSSProperties = {
    position: 'fixed',
    inset: 0,
    backgroundColor: 'rgba(6, 11, 10, 0.85)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    zIndex: 'var(--z-modal-backdrop)' as unknown as number,
    animation: 'fadeIn 200ms ease',
  };

  const dialogStyle: CSSProperties = {
    backgroundColor: 'var(--color-surface-800)',
    border: '1px solid var(--color-border-700)',
    borderRadius: 'var(--radius-xl)',
    width,
    maxWidth: '90vw',
    maxHeight: '85vh',
    display: 'flex',
    flexDirection: 'column',
    boxShadow: 'var(--shadow-lg)',
    zIndex: 'var(--z-modal)' as unknown as number,
    animation: 'slideUp 250ms ease',
    overflow: 'hidden',
  };

  const headerStyle: CSSProperties = {
    padding: 'var(--space-5) var(--space-6)',
    borderBottom: title ? '1px solid var(--color-border-700)' : 'none',
  };

  const titleStyle: CSSProperties = {
    fontSize: 'var(--text-lg)',
    fontWeight: 'var(--weight-bold)' as unknown as number,
    color: 'var(--color-text-primary)',
  };

  const bodyStyle: CSSProperties = {
    padding: 'var(--space-5) var(--space-6)',
    overflowY: 'auto',
    flex: 1,
  };

  const footerStyle: CSSProperties = {
    padding: 'var(--space-4) var(--space-6)',
    borderTop: '1px solid var(--color-border-700)',
    display: 'flex',
    justifyContent: 'flex-end',
    gap: 'var(--space-3)',
  };

  return (
    <div style={backdropStyle} onClick={onClose} role="dialog" aria-modal="true">
      <div style={dialogStyle} onClick={(e) => e.stopPropagation()}>
        {title && (
          <div style={headerStyle}>
            <h2 style={titleStyle}>{title}</h2>
          </div>
        )}
        <div style={bodyStyle}>{children}</div>
        {footer && <div style={footerStyle}>{footer}</div>}
      </div>
    </div>
  );
}
