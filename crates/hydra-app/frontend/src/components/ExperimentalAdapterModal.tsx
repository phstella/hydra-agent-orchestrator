import { useState } from 'react';
import { Modal, Button, Badge, Card, ProgressBar } from './design-system';
import type { AdapterInfo } from '../types';

interface ExperimentalAdapterModalProps {
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
  adapter: AdapterInfo | null;
}

export function ExperimentalAdapterModal({
  open,
  onClose,
  onConfirm,
  adapter,
}: ExperimentalAdapterModalProps) {
  const [acknowledged, setAcknowledged] = useState(false);

  const handleClose = () => {
    setAcknowledged(false);
    onClose();
  };

  const handleConfirm = () => {
    if (!acknowledged) return;
    setAcknowledged(false);
    onConfirm();
  };

  if (!adapter) return null;

  return (
    <Modal
      open={open}
      onClose={handleClose}
      width="500px"
      footer={
        <>
          <Button variant="secondary" onClick={handleClose}>
            Cancel
          </Button>
          <Button
            variant="primary"
            disabled={!acknowledged}
            onClick={handleConfirm}
            style={
              acknowledged
                ? { backgroundColor: 'var(--color-marine-500)' }
                : undefined
            }
          >
            Confirm Selection
          </Button>
        </>
      }
    >
      <div style={{ display: 'flex', alignItems: 'flex-start', gap: 'var(--space-4)', marginBottom: 'var(--space-5)' }}>
        {/* Warning icon */}
        <span
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            width: 48,
            height: 48,
            borderRadius: '50%',
            backgroundColor: 'rgba(234, 179, 8, 0.15)',
            color: 'var(--color-warning-500)',
            fontSize: 'var(--text-xl)',
            flexShrink: 0,
          }}
        >
          ⚠
        </span>
        <div>
          <h2 style={{ fontSize: 'var(--text-xl)', fontWeight: 'var(--weight-bold)' as unknown as number, marginBottom: 'var(--space-2)' }}>
            Experimental Adapter Warning
          </h2>
          <p style={{ color: 'var(--color-text-muted)', fontSize: 'var(--text-sm)', lineHeight: 'var(--leading-relaxed)' }}>
            You are selecting an experimental adapter (
            <strong style={{ color: 'var(--color-text-secondary)' }}>{adapter.key}</strong>
            ). These agents may produce unstable results or consume excessive local resources.
          </p>
        </div>
      </div>

      {/* Resource impact card */}
      <Card variant="outlined" padding="md" style={{ marginBottom: 'var(--space-5)' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)', marginBottom: 'var(--space-3)' }}>
          <span style={{ color: 'var(--color-text-muted)' }}>⚙</span>
          <span
            style={{
              fontSize: 'var(--text-xs)',
              fontWeight: 'var(--weight-semibold)' as unknown as number,
              color: 'var(--color-text-secondary)',
              textTransform: 'uppercase',
              letterSpacing: '0.05em',
            }}
          >
            Potential Resource Impact
          </span>
        </div>
        <ProgressBar value={70} variant="gradient" height={10} />
        <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: 'var(--space-2)' }}>
          <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>
            Low Usage
          </span>
          <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-danger-400)' }}>
            High Load Detected
          </span>
        </div>
      </Card>

      {/* Adapter info badges */}
      <div style={{ display: 'flex', gap: 'var(--space-2)', marginBottom: 'var(--space-5)' }}>
        <Badge variant="experimental">Experimental</Badge>
        <Badge variant="neutral">
          {adapter.confidence === 'verified' ? 'Verified' : adapter.confidence === 'observed' ? 'Observed' : 'Unverified'}
        </Badge>
        {adapter.version && <Badge variant="info">{adapter.version}</Badge>}
      </div>

      {/* Risk acknowledgment */}
      <label
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--space-3)',
          cursor: 'pointer',
          padding: 'var(--space-3)',
          borderRadius: 'var(--radius-md)',
          backgroundColor: acknowledged ? 'rgba(47, 111, 159, 0.1)' : 'transparent',
          border: '1px solid',
          borderColor: acknowledged ? 'var(--color-marine-500)' : 'var(--color-border-700)',
          transition: 'all var(--transition-fast)',
        }}
      >
        <input
          type="checkbox"
          checked={acknowledged}
          onChange={(e) => setAcknowledged(e.target.checked)}
          style={{
            width: 18,
            height: 18,
            accentColor: 'var(--color-marine-500)',
            flexShrink: 0,
          }}
        />
        <span style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-primary)' }}>
          I understand the risks and want to proceed.
        </span>
      </label>
    </Modal>
  );
}
