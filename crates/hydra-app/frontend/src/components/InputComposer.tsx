import { useState, useCallback, useRef } from 'react';
import type { CSSProperties, KeyboardEvent } from 'react';
import { Button } from './design-system';

interface InputComposerProps {
  sessionId: string | null;
  sessionStatus: string | null;
  onSendInput: (input: string) => Promise<{ success: boolean; error: string | null }>;
  onStopSession: () => void;
}

export function InputComposer({
  sessionId,
  sessionStatus,
  onSendInput,
  onStopSession,
}: InputComposerProps) {
  const [input, setInput] = useState('');
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const isRunning = sessionStatus === 'running';
  const canSend = !!sessionId && isRunning && input.trim().length > 0 && !sending;

  const handleSend = useCallback(async () => {
    if (!canSend) return;
    const trimmed = input.trim();
    setError(null);
    setSending(true);

    try {
      const result = await onSendInput(trimmed);
      if (result.success) {
        setInput('');
        inputRef.current?.focus();
      } else {
        setError(result.error ?? 'Failed to send input');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSending(false);
    }
  }, [canSend, input, onSendInput]);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend],
  );

  if (!sessionId) return null;

  const containerStyle: CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    gap: 'var(--space-2)',
    padding: 'var(--space-3) var(--space-4)',
    borderTop: '1px solid var(--color-border-700)',
    backgroundColor: 'var(--color-bg-900)',
  };

  const inputRowStyle: CSSProperties = {
    display: 'flex',
    gap: 'var(--space-2)',
    alignItems: 'flex-end',
  };

  const textareaStyle: CSSProperties = {
    flex: 1,
    borderRadius: 'var(--radius-md)',
    border: '1px solid var(--color-border-700)',
    backgroundColor: 'var(--color-bg-950)',
    color: isRunning ? 'var(--color-text-primary)' : 'var(--color-text-muted)',
    padding: 'var(--space-2) var(--space-3)',
    resize: 'none',
    fontFamily: 'var(--font-mono)',
    fontSize: 'var(--text-sm)',
    lineHeight: 'var(--leading-normal)',
    minHeight: 38,
    maxHeight: 120,
    opacity: isRunning ? 1 : 0.5,
  };

  return (
    <div style={containerStyle} data-testid="input-composer">
      <div style={inputRowStyle}>
        <textarea
          ref={inputRef}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={isRunning ? 'Send input to agent... (Enter to send, Shift+Enter for newline)' : 'Session is not running'}
          disabled={!isRunning}
          rows={1}
          style={textareaStyle}
          data-testid="interactive-input"
        />
        <Button
          variant="primary"
          size="sm"
          onClick={handleSend}
          disabled={!canSend}
          loading={sending}
          data-testid="send-input-btn"
        >
          Send
        </Button>
        {isRunning && (
          <Button
            variant="danger"
            size="sm"
            onClick={onStopSession}
            data-testid="stop-session-btn"
          >
            Stop
          </Button>
        )}
        {!isRunning && sessionStatus && sessionStatus !== 'unknown' && (
          <span
            style={{
              fontSize: 'var(--text-xs)',
              color: 'var(--color-text-muted)',
              alignSelf: 'center',
              whiteSpace: 'nowrap',
            }}
            data-testid="session-ended-indicator"
          >
            Session {sessionStatus}
          </span>
        )}
      </div>
      {error && (
        <div
          style={{
            fontSize: 'var(--text-xs)',
            color: 'var(--color-danger-400)',
          }}
          data-testid="input-error"
        >
          {error}
        </div>
      )}
    </div>
  );
}
