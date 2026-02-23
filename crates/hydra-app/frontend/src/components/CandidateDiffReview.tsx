import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import type { CSSProperties } from 'react';
import type {
  AgentResult,
  CandidateDiffPayload,
  MergePreviewPayload,
  MergeExecutionPayload,
  DiffFile,
} from '../types';
import { getCandidateDiff, previewMerge, executeMerge } from '../ipc';
import { Badge, Button, Card, Panel, Modal } from './design-system';

interface CandidateDiffReviewProps {
  runId: string;
  agents: AgentResult[];
  selectedWinner: string | null;
}

type MergeStatus = 'idle' | 'previewing' | 'preview_clean' | 'preview_conflict' | 'merging' | 'merged' | 'failed';

export function CandidateDiffReview({ runId, agents, selectedWinner }: CandidateDiffReviewProps) {
  const [activeCandidate, setActiveCandidate] = useState<string>(selectedWinner ?? agents[0]?.agentKey ?? '');
  const [diffPayload, setDiffPayload] = useState<CandidateDiffPayload | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);
  const [diffError, setDiffError] = useState<string | null>(null);

  const [mergeStatus, setMergeStatus] = useState<MergeStatus>('idle');
  const [mergePreview, setMergePreview] = useState<MergePreviewPayload | null>(null);
  const [mergeResult, setMergeResult] = useState<MergeExecutionPayload | null>(null);
  const [mergeError, setMergeError] = useState<string | null>(null);
  const [forceOverride, setForceOverride] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);

  useEffect(() => {
    if (selectedWinner && agents.some((a) => a.agentKey === selectedWinner)) {
      setActiveCandidate(selectedWinner);
    }
  }, [selectedWinner, agents]);

  const loadDiff = useCallback(
    async (agentKey: string) => {
      setDiffLoading(true);
      setDiffError(null);
      setMergeStatus('idle');
      setMergePreview(null);
      setMergeResult(null);
      setMergeError(null);
      try {
        const payload = await getCandidateDiff(runId, agentKey);
        setDiffPayload(payload);
      } catch (err) {
        setDiffError(err instanceof Error ? err.message : String(err));
        setDiffPayload(null);
      } finally {
        setDiffLoading(false);
      }
    },
    [runId],
  );

  useEffect(() => {
    if (activeCandidate) {
      loadDiff(activeCandidate);
    }
  }, [activeCandidate, loadDiff]);

  const activeAgent = useMemo(
    () => agents.find((a) => a.agentKey === activeCandidate),
    [agents, activeCandidate],
  );

  const isMergeable = diffPayload?.mergeable === true && (diffPayload?.gateFailures.length ?? 0) === 0;
  const canMerge = isMergeable || forceOverride;

  const handlePreview = useCallback(async () => {
    setMergeStatus('previewing');
    setMergeError(null);
    try {
      const result = await previewMerge(runId, activeCandidate, forceOverride);
      setMergePreview(result);
      setMergeStatus(result.hasConflicts ? 'preview_conflict' : 'preview_clean');
    } catch (err) {
      setMergeError(err instanceof Error ? err.message : String(err));
      setMergeStatus('failed');
    }
  }, [runId, activeCandidate, forceOverride]);

  const handleAccept = useCallback(() => {
    setConfirmOpen(true);
  }, []);

  const handleConfirmMerge = useCallback(async () => {
    setConfirmOpen(false);
    setMergeStatus('merging');
    setMergeError(null);
    try {
      const result = await executeMerge(runId, activeCandidate, forceOverride);
      setMergeResult(result);
      setMergeStatus(result.success ? 'merged' : 'failed');
      if (!result.success) {
        setMergeError(result.message);
      }
    } catch (err) {
      setMergeError(err instanceof Error ? err.message : String(err));
      setMergeStatus('failed');
    }
  }, [runId, activeCandidate, forceOverride]);

  const containerStyle: CSSProperties = {
    maxWidth: 1200,
    margin: '0 auto',
    padding: 'var(--space-6)',
    display: 'flex',
    flexDirection: 'column',
    gap: 'var(--space-4)',
  };

  return (
    <div style={containerStyle}>
      <div style={{ marginBottom: 'var(--space-2)' }}>
        <h1
          style={{
            fontSize: 'var(--text-2xl)',
            fontWeight: 'var(--weight-bold)' as unknown as number,
            color: 'var(--color-text-primary)',
            marginBottom: 'var(--space-2)',
          }}
        >
          Diff Review
        </h1>
        <div style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-secondary)' }}>
          Review candidate changes and merge the winner
        </div>
      </div>

      <CandidateTabBar
        agents={agents}
        activeCandidate={activeCandidate}
        selectedWinner={selectedWinner}
        onSelect={setActiveCandidate}
      />

      {diffLoading && (
        <Card padding="lg">
          <div style={{ textAlign: 'center', color: 'var(--color-text-muted)', padding: 'var(--space-8)' }}>
            Loading diff for {activeCandidate}...
          </div>
        </Card>
      )}

      {diffError && (
        <Card padding="lg">
          <div style={{ color: 'var(--color-danger-400)', padding: 'var(--space-4)' }}>
            Failed to load diff: {diffError}
          </div>
        </Card>
      )}

      {!diffLoading && !diffError && diffPayload && !diffPayload.diffAvailable && (
        <Card padding="lg">
          <div style={{ textAlign: 'center', padding: 'var(--space-8)' }}>
            <div
              style={{
                fontSize: 'var(--text-lg)',
                color: 'var(--color-warning-400)',
                marginBottom: 'var(--space-3)',
              }}
            >
              Diff Unavailable
            </div>
            <div style={{ color: 'var(--color-text-muted)', fontSize: 'var(--text-sm)' }}>
              {diffPayload.warning ?? 'The diff artifact was not persisted and the branch no longer exists.'}
            </div>
          </div>
        </Card>
      )}

      {!diffLoading && !diffError && diffPayload?.diffAvailable && (
        <div style={{ display: 'flex', gap: 'var(--space-4)', minHeight: 500 }}>
          <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 'var(--space-4)' }}>
            {diffPayload.warning && (
              <div
                style={{
                  padding: 'var(--space-2) var(--space-3)',
                  backgroundColor: 'color-mix(in srgb, var(--color-warning-500) 10%, transparent)',
                  border: '1px solid var(--color-warning-500)',
                  borderRadius: 'var(--radius-md)',
                  fontSize: 'var(--text-xs)',
                  color: 'var(--color-warning-400)',
                }}
              >
                {diffPayload.warning}
              </div>
            )}
            <DiffViewerPane diffText={diffPayload.diffText} />
          </div>
          <div style={{ width: 280, flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 'var(--space-4)' }}>
            <ModifiedFilesList files={diffPayload.files} />
            <MergeActionRail
              agent={activeAgent ?? null}
              diffPayload={diffPayload}
              mergeStatus={mergeStatus}
              mergePreview={mergePreview}
              mergeResult={mergeResult}
              mergeError={mergeError}
              canMerge={canMerge}
              forceOverride={forceOverride}
              onForceToggle={setForceOverride}
              onPreview={handlePreview}
              onAccept={handleAccept}
              onReject={() => { setMergeStatus('idle'); setMergePreview(null); setMergeResult(null); }}
            />
          </div>
        </div>
      )}

      <Modal
        open={confirmOpen}
        onClose={() => setConfirmOpen(false)}
        title="Confirm Merge"
        footer={
          <>
            <Button variant="ghost" onClick={() => setConfirmOpen(false)}>Cancel</Button>
            <Button variant="primary" onClick={handleConfirmMerge}>
              Confirm Merge
            </Button>
          </>
        }
      >
        <div style={{ color: 'var(--color-text-secondary)', fontSize: 'var(--text-sm)' }}>
          <p style={{ marginBottom: 'var(--space-3)' }}>
            Merge <strong style={{ color: 'var(--color-text-primary)' }}>{activeCandidate}</strong> into the current branch?
          </p>
          {forceOverride && (
            <div
              style={{
                padding: 'var(--space-3)',
                backgroundColor: 'color-mix(in srgb, var(--color-warning-500) 10%, transparent)',
                border: '1px solid var(--color-warning-500)',
                borderRadius: 'var(--radius-md)',
                marginBottom: 'var(--space-3)',
              }}
            >
              <strong style={{ color: 'var(--color-warning-400)' }}>Force override is enabled.</strong>{' '}
              Mergeability gates are being bypassed.
            </div>
          )}
          <p>This action cannot be undone from the UI. You can revert with git if needed.</p>
        </div>
      </Modal>
    </div>
  );
}

function CandidateTabBar({
  agents,
  activeCandidate,
  selectedWinner,
  onSelect,
}: {
  agents: AgentResult[];
  activeCandidate: string;
  selectedWinner: string | null;
  onSelect: (key: string) => void;
}) {
  const tabBarStyle: CSSProperties = {
    display: 'flex',
    gap: 'var(--space-1)',
    borderBottom: '1px solid var(--color-border-700)',
    paddingBottom: 0,
  };

  return (
    <div style={tabBarStyle} role="tablist" data-testid="candidate-tabs">
      {agents.map((agent) => {
        const active = activeCandidate === agent.agentKey;
        const isWinner = selectedWinner === agent.agentKey;
        return (
          <button
            key={agent.agentKey}
            role="tab"
            aria-selected={active}
            data-testid={`candidate-tab-${agent.agentKey}`}
            onClick={() => onSelect(agent.agentKey)}
            style={{
              padding: 'var(--space-2) var(--space-4)',
              fontSize: 'var(--text-sm)',
              fontWeight: active ? ('var(--weight-semibold)' as unknown as number) : ('var(--weight-normal)' as unknown as number),
              color: active ? 'var(--color-marine-400)' : 'var(--color-text-muted)',
              background: 'transparent',
              border: 'none',
              borderBottom: active ? '2px solid var(--color-marine-500)' : '2px solid transparent',
              cursor: 'pointer',
              fontFamily: 'var(--font-family)',
              display: 'flex',
              alignItems: 'center',
              gap: 'var(--space-2)',
              transition: 'all var(--transition-fast)',
            }}
          >
            {agent.agentKey}
            {isWinner && <Badge variant="success">WINNER</Badge>}
            {agent.score !== null && (
              <Badge variant="neutral">{Math.round(agent.score)}</Badge>
            )}
          </button>
        );
      })}
    </div>
  );
}

function DiffViewerPane({ diffText }: { diffText: string }) {
  const containerRef = useRef<HTMLDivElement>(null);

  const TAIL_LINES = 5000;
  const lines = useMemo(() => {
    const all = diffText.split('\n');
    if (all.length > TAIL_LINES) {
      return all.slice(0, TAIL_LINES);
    }
    return all;
  }, [diffText]);

  const truncated = diffText.split('\n').length > TAIL_LINES;

  return (
    <Panel title="Diff" headerRight={<Badge variant="neutral">{diffText.split('\n').length} lines</Badge>}>
      <div
        ref={containerRef}
        data-testid="diff-viewer"
        style={{
          fontFamily: 'var(--font-mono)',
          fontSize: 'var(--text-xs)',
          lineHeight: 'var(--leading-relaxed)',
          overflowX: 'auto',
          overflowY: 'auto',
          maxHeight: 500,
          whiteSpace: 'pre',
          tabSize: 4,
        }}
      >
        {truncated && (
          <div style={{ color: 'var(--color-warning-400)', padding: 'var(--space-2)', borderBottom: '1px solid var(--color-border-700)' }}>
            Showing first {TAIL_LINES} of {diffText.split('\n').length} lines
          </div>
        )}
        {lines.map((line, i) => (
          <DiffLine key={i} line={line} />
        ))}
      </div>
    </Panel>
  );
}

function DiffLine({ line }: { line: string }) {
  let color = 'var(--color-text-secondary)';
  let bg = 'transparent';

  if (line.startsWith('+') && !line.startsWith('+++')) {
    color = 'var(--color-green-400)';
    bg = 'color-mix(in srgb, var(--color-green-500) 6%, transparent)';
  } else if (line.startsWith('-') && !line.startsWith('---')) {
    color = 'var(--color-danger-400)';
    bg = 'color-mix(in srgb, var(--color-danger-500) 6%, transparent)';
  } else if (line.startsWith('@@')) {
    color = 'var(--color-marine-400)';
    bg = 'color-mix(in srgb, var(--color-marine-500) 8%, transparent)';
  } else if (line.startsWith('diff --git')) {
    color = 'var(--color-text-primary)';
    bg = 'color-mix(in srgb, var(--color-text-muted) 8%, transparent)';
  }

  return (
    <div style={{ color, backgroundColor: bg, paddingLeft: 'var(--space-2)', paddingRight: 'var(--space-2)', minHeight: '1.5em' }}>
      {line || ' '}
    </div>
  );
}

function ModifiedFilesList({ files }: { files: DiffFile[] }) {
  return (
    <Panel title="Modified Files" headerRight={<Badge variant="neutral">{files.length}</Badge>}>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-1)' }} data-testid="modified-files">
        {files.length === 0 && (
          <div style={{ color: 'var(--color-text-muted)', fontSize: 'var(--text-xs)' }}>No files modified</div>
        )}
        {files.map((file) => (
          <div
            key={file.path}
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              padding: 'var(--space-1) var(--space-2)',
              borderRadius: 'var(--radius-sm)',
              fontSize: 'var(--text-xs)',
              fontFamily: 'var(--font-mono)',
            }}
          >
            <span style={{ color: 'var(--color-text-secondary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {file.path}
            </span>
            <span style={{ flexShrink: 0, marginLeft: 'var(--space-2)' }}>
              <span style={{ color: 'var(--color-green-400)' }}>+{file.added}</span>
              {' '}
              <span style={{ color: 'var(--color-danger-400)' }}>-{file.removed}</span>
            </span>
          </div>
        ))}
      </div>
    </Panel>
  );
}

function MergeActionRail({
  agent,
  diffPayload,
  mergeStatus,
  mergePreview,
  mergeResult,
  mergeError,
  canMerge,
  forceOverride,
  onForceToggle,
  onPreview,
  onAccept,
  onReject,
}: {
  agent: AgentResult | null;
  diffPayload: CandidateDiffPayload;
  mergeStatus: MergeStatus;
  mergePreview: MergePreviewPayload | null;
  mergeResult: MergeExecutionPayload | null;
  mergeError: string | null;
  canMerge: boolean;
  forceOverride: boolean;
  onForceToggle: (v: boolean) => void;
  onPreview: () => void;
  onAccept: () => void;
  onReject: () => void;
}) {
  const statusBadge = useMemo(() => {
    if (mergeStatus === 'merged') return { variant: 'success' as const, label: 'MERGED' };
    if (mergeStatus === 'preview_conflict') return { variant: 'danger' as const, label: 'CONFLICT' };
    if (mergeStatus === 'preview_clean') return { variant: 'success' as const, label: 'CLEAN' };
    if (diffPayload.mergeable === false) return { variant: 'danger' as const, label: 'NOT MERGEABLE' };
    if ((diffPayload.gateFailures?.length ?? 0) > 0) return { variant: 'warning' as const, label: 'GATED' };
    if (diffPayload.mergeable === true) return { variant: 'success' as const, label: 'MERGEABLE' };
    return { variant: 'neutral' as const, label: 'UNKNOWN' };
  }, [mergeStatus, diffPayload]);

  return (
    <Panel title="Merge Actions">
      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-3)' }} data-testid="merge-rail">
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
          <Badge variant={statusBadge.variant} dot>{statusBadge.label}</Badge>
          {agent && agent.score !== null && (
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>
              Score: {Math.round(agent.score)}
            </span>
          )}
        </div>

        {diffPayload.gateFailures.length > 0 && (
          <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-danger-400)' }}>
            {diffPayload.gateFailures.map((f, i) => (
              <div key={i}>Gate: {f}</div>
            ))}
          </div>
        )}

        {!canMerge && diffPayload.mergeable === false && (
          <label
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 'var(--space-2)',
              fontSize: 'var(--text-xs)',
              color: 'var(--color-warning-400)',
              cursor: 'pointer',
            }}
          >
            <input
              type="checkbox"
              checked={forceOverride}
              onChange={(e) => onForceToggle(e.target.checked)}
              data-testid="force-override"
            />
            Force override (bypass gates)
          </label>
        )}

        <Button
          variant="secondary"
          size="sm"
          style={{ width: '100%' }}
          onClick={onPreview}
          disabled={mergeStatus === 'previewing' || mergeStatus === 'merging' || mergeStatus === 'merged'}
          loading={mergeStatus === 'previewing'}
          data-testid="preview-merge-btn"
        >
          Preview Merge
        </Button>

        {mergePreview && (
          <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-secondary)' }}>
            {mergePreview.hasConflicts
              ? <span style={{ color: 'var(--color-danger-400)' }}>Conflicts detected. Merge cannot proceed.</span>
              : <span style={{ color: 'var(--color-green-400)' }}>Clean merge. No conflicts.</span>}
          </div>
        )}

        <Button
          variant="primary"
          size="sm"
          style={{ width: '100%' }}
          onClick={onAccept}
          disabled={!canMerge || mergeStatus === 'merging' || mergeStatus === 'merged' || mergeStatus === 'preview_conflict'}
          loading={mergeStatus === 'merging'}
          data-testid="accept-merge-btn"
        >
          {mergeStatus === 'merged' ? 'Merged' : 'Accept Candidate'}
        </Button>

        <Button
          variant="ghost"
          size="sm"
          style={{ width: '100%' }}
          onClick={onReject}
          disabled={mergeStatus === 'merging' || mergeStatus === 'merged'}
          data-testid="reject-btn"
        >
          Reject
        </Button>

        {mergeResult?.success && (
          <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-green-400)' }}>
            {mergeResult.message}
          </div>
        )}

        {mergeError && (
          <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-danger-400)' }}>
            {mergeError}
          </div>
        )}
      </div>
    </Panel>
  );
}
