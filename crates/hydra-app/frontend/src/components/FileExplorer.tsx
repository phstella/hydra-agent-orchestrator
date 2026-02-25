import { useState, useEffect, useCallback, useRef } from 'react';
import type { CSSProperties } from 'react';
import { Button } from './design-system';
import {
  listDirectory,
  startFileWatcher,
  pollFileWatchEvents,
  stopFileWatcher,
} from '../ipc';
import type { FileTreeEntry, FileWatchEvent } from '../types';

const WATCH_POLL_INTERVAL_MS = 1_000;
const WATCH_POLL_RETRY_MS = 3_000;
const DEBOUNCE_REFRESH_MS = 300;

function normalizePath(path: string): string {
  return path.replace(/\\/g, '/');
}

function parentDirectory(path: string, fallback: string): string {
  const lastSlash = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
  if (lastSlash <= 0) return fallback;
  return path.slice(0, lastSlash);
}

interface FileExplorerProps {
  workspaceCwd: string | null;
}

export function FileExplorer({ workspaceCwd }: FileExplorerProps) {
  const [rootEntries, setRootEntries] = useState<FileTreeEntry[]>([]);
  const [expanded, setExpanded] = useState<Map<string, FileTreeEntry[]>>(new Map());
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [watcherId, setWatcherId] = useState<string | null>(null);
  const [watchError, setWatchError] = useState<string | null>(null);
  const pollTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pollCursor = useRef(0);
  const debounceTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const effectivePath = workspaceCwd ?? '.';

  // -------------------------------------------------------------------------
  // Load root directory
  // -------------------------------------------------------------------------
  const loadRoot = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const listing = await listDirectory(effectivePath);
      if (listing.error) {
        setError(listing.error);
        setRootEntries([]);
      } else {
        setRootEntries(listing.entries);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setRootEntries([]);
    } finally {
      setLoading(false);
    }
  }, [effectivePath]);

  // -------------------------------------------------------------------------
  // Load a subdirectory (lazy expand)
  // -------------------------------------------------------------------------
  const loadSubdir = useCallback(async (dirPath: string) => {
    try {
      const listing = await listDirectory(dirPath);
      if (!listing.error) {
        setExpanded((prev) => {
          const next = new Map(prev);
          next.set(dirPath, listing.entries);
          return next;
        });
      }
    } catch {
      // best effort
    }
  }, []);

  // -------------------------------------------------------------------------
  // Toggle expand/collapse
  // -------------------------------------------------------------------------
  const toggleDir = useCallback(
    (dirPath: string) => {
      if (expanded.has(dirPath)) {
        setExpanded((prev) => {
          const next = new Map(prev);
          next.delete(dirPath);
          return next;
        });
      } else {
        loadSubdir(dirPath);
      }
    },
    [expanded, loadSubdir],
  );

  // -------------------------------------------------------------------------
  // Manual refresh
  // -------------------------------------------------------------------------
  const handleRefresh = useCallback(() => {
    setExpanded(new Map());
    loadRoot();
  }, [loadRoot]);

  // -------------------------------------------------------------------------
  // Debounced refresh for watcher events
  // -------------------------------------------------------------------------
  const debouncedRefresh = useCallback(
    (events: FileWatchEvent[]) => {
      if (events.length === 0) return;

      // Determine which directories need refreshing
      const dirsToRefresh = new Set<string>();
      for (const evt of events) {
        const parentDir = parentDirectory(evt.path, effectivePath);
        if (normalizePath(parentDir) === normalizePath(effectivePath)) {
          dirsToRefresh.add('__root__');
        }
        if (expanded.has(parentDir)) {
          dirsToRefresh.add(parentDir);
        } else {
          const altParentDir = parentDir.includes('\\')
            ? parentDir.replace(/\\/g, '/')
            : parentDir.replace(/\//g, '\\');
          if (expanded.has(altParentDir)) {
            dirsToRefresh.add(altParentDir);
          }
        }
      }

      if (dirsToRefresh.size === 0) return;

      if (debounceTimer.current) clearTimeout(debounceTimer.current);
      debounceTimer.current = setTimeout(() => {
        if (dirsToRefresh.has('__root__')) {
          loadRoot();
        }
        for (const dir of dirsToRefresh) {
          if (dir !== '__root__') {
            loadSubdir(dir);
          }
        }
      }, DEBOUNCE_REFRESH_MS);
    },
    [effectivePath, expanded, loadRoot, loadSubdir],
  );

  // -------------------------------------------------------------------------
  // File watcher lifecycle
  // -------------------------------------------------------------------------
  useEffect(() => {
    if (!effectivePath) return;

    setWatcherId(null);
    setWatchError(null);
    pollCursor.current = 0;

    let cancelled = false;
    let currentWatcherId: string | null = null;

    async function startWatcher() {
      try {
        const result = await startFileWatcher(effectivePath);
        if (cancelled) {
          await stopFileWatcher(result.watcherId).catch(() => {});
          return;
        }
        currentWatcherId = result.watcherId;
        setWatcherId(result.watcherId);
        setWatchError(null);
        startPolling(result.watcherId);
      } catch (err) {
        if (!cancelled) {
          setWatchError(err instanceof Error ? err.message : String(err));
        }
      }
    }

    function startPolling(wid: string) {
      async function poll() {
        if (cancelled) return;
        try {
          const batch = await pollFileWatchEvents(wid, pollCursor.current);
          if (cancelled) return;

          pollCursor.current = batch.nextCursor;

          if (batch.events.length > 0) {
            debouncedRefresh(batch.events);
          }

          if (batch.error) {
            setWatchError(batch.error);
          } else {
            setWatchError(null);
          }

          if (!batch.active) {
            setWatcherId((current) => (current === wid ? null : current));
            return;
          }

          pollTimer.current = setTimeout(poll, WATCH_POLL_INTERVAL_MS);
        } catch {
          if (!cancelled) {
            pollTimer.current = setTimeout(poll, WATCH_POLL_RETRY_MS);
          }
        }
      }

      poll();
    }

    startWatcher();

    return () => {
      cancelled = true;
      if (pollTimer.current) {
        clearTimeout(pollTimer.current);
        pollTimer.current = null;
      }
      if (debounceTimer.current) {
        clearTimeout(debounceTimer.current);
        debounceTimer.current = null;
      }
      pollCursor.current = 0;
      if (currentWatcherId) {
        stopFileWatcher(currentWatcherId).catch(() => {});
      }
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [effectivePath]);

  // -------------------------------------------------------------------------
  // Initial load
  // -------------------------------------------------------------------------
  useEffect(() => {
    loadRoot();
  }, [loadRoot]);

  // -------------------------------------------------------------------------
  // Render
  // -------------------------------------------------------------------------
  const containerStyle: CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    flex: 1,
    minHeight: 0,
    overflow: 'hidden',
  };

  const headerStyle: CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: 'var(--space-3) var(--space-4)',
    borderBottom: '1px solid var(--color-border-700)',
    backgroundColor: 'var(--color-bg-900)',
    flexShrink: 0,
  };

  const titleStyle: CSSProperties = {
    fontSize: 'var(--text-sm)',
    fontWeight: 'var(--weight-semibold)' as unknown as number,
    color: 'var(--color-text-primary)',
  };

  const pathStyle: CSSProperties = {
    fontSize: 'var(--text-xs)',
    color: 'var(--color-text-muted)',
    fontFamily: 'var(--font-mono)',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  };

  const treeAreaStyle: CSSProperties = {
    flex: 1,
    overflowY: 'auto',
    padding: 'var(--space-2)',
    backgroundColor: 'var(--color-bg-950)',
    fontFamily: 'var(--font-mono)',
    fontSize: 'var(--text-xs)',
  };

  return (
    <div style={containerStyle} data-testid="file-explorer">
      <div style={headerStyle}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-1)', minWidth: 0, flex: 1 }}>
          <span style={titleStyle}>File Explorer</span>
          <span style={pathStyle} data-testid="file-explorer-root">
            {effectivePath}
          </span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)', flexShrink: 0 }}>
          {watcherId && !watchError && (
            <span
              style={{ fontSize: 'var(--text-xs)', color: 'var(--color-green-400)' }}
              data-testid="watcher-active-indicator"
            >
              watching
            </span>
          )}
          <Button
            variant="secondary"
            size="sm"
            onClick={handleRefresh}
            loading={loading}
            data-testid="file-explorer-refresh"
          >
            Refresh
          </Button>
        </div>
      </div>

      {error && (
        <div
          style={{
            padding: 'var(--space-3) var(--space-4)',
            color: 'var(--color-danger-400)',
            fontSize: 'var(--text-xs)',
            backgroundColor: 'color-mix(in srgb, var(--color-danger-500) 8%, transparent)',
            borderBottom: '1px solid var(--color-border-700)',
          }}
          data-testid="file-explorer-error"
        >
          {error}
        </div>
      )}

      {watchError && (
        <div
          style={{
            padding: 'var(--space-2) var(--space-4)',
            color: 'var(--color-warning-400)',
            fontSize: 'var(--text-xs)',
            backgroundColor: 'color-mix(in srgb, var(--color-warning-500) 8%, transparent)',
            borderBottom: '1px solid var(--color-border-700)',
          }}
          data-testid="file-explorer-watch-error"
        >
          Watcher: {watchError}
        </div>
      )}

      <div style={treeAreaStyle} data-testid="file-tree">
        {loading && rootEntries.length === 0 && (
          <div style={{ color: 'var(--color-text-muted)', padding: 'var(--space-4)', textAlign: 'center' }}>
            Loading...
          </div>
        )}

        {!loading && rootEntries.length === 0 && !error && (
          <div
            style={{ color: 'var(--color-text-muted)', padding: 'var(--space-4)', textAlign: 'center' }}
            data-testid="file-tree-empty"
          >
            No files found.
          </div>
        )}

        {rootEntries.map((entry) => (
          <TreeNode
            key={entry.path}
            entry={entry}
            depth={0}
            expanded={expanded}
            onToggle={toggleDir}
          />
        ))}
      </div>
    </div>
  );
}

function TreeNode({
  entry,
  depth,
  expanded,
  onToggle,
}: {
  entry: FileTreeEntry;
  depth: number;
  expanded: Map<string, FileTreeEntry[]>;
  onToggle: (path: string) => void;
}) {
  const isDir = entry.entryType === 'directory';
  const isExpanded = expanded.has(entry.path);
  const children = isExpanded ? expanded.get(entry.path) ?? [] : [];

  const nodeStyle: CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    gap: 'var(--space-1)',
    padding: '2px var(--space-1)',
    paddingLeft: `calc(var(--space-3) * ${depth} + var(--space-1))`,
    cursor: isDir ? 'pointer' : 'default',
    color: isDir ? 'var(--color-text-primary)' : 'var(--color-text-secondary)',
    borderRadius: 'var(--radius-sm)',
  };

  const iconStyle: CSSProperties = {
    flexShrink: 0,
    width: 16,
    textAlign: 'center',
    fontSize: 'var(--text-xs)',
    color: isDir
      ? isExpanded
        ? 'var(--color-marine-400)'
        : 'var(--color-text-muted)'
      : 'var(--color-text-muted)',
  };

  return (
    <>
      <div
        style={nodeStyle}
        onClick={isDir ? () => onToggle(entry.path) : undefined}
        data-testid={`tree-node-${entry.name}`}
      >
        <span style={iconStyle}>
          {isDir ? (isExpanded ? '▾' : '▸') : '·'}
        </span>
        <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
          {entry.name}
        </span>
        {entry.size !== null && (
          <span style={{ marginLeft: 'auto', color: 'var(--color-text-muted)', fontSize: '10px', flexShrink: 0 }}>
            {formatSize(entry.size)}
          </span>
        )}
      </div>
      {isExpanded &&
        children.map((child) => (
          <TreeNode
            key={child.path}
            entry={child}
            depth={depth + 1}
            expanded={expanded}
            onToggle={onToggle}
          />
        ))}
    </>
  );
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
