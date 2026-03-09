import { useState, useEffect, useCallback, useRef } from 'react';
import type { CSSProperties, ReactNode } from 'react';
import { Button } from './design-system';
import {
  listDirectory,
  readFilePreview,
  startFileWatcher,
  pollFileWatchEvents,
  stopFileWatcher,
} from '../ipc';
import type { FileTreeEntry, FileWatchEvent, FilePreview } from '../types';

const WATCH_POLL_INTERVAL_MS = 1_000;
const WATCH_POLL_RETRY_MS = 3_000;
const DEBOUNCE_REFRESH_MS = 300;
const DEFAULT_PREVIEW_BYTES = 96 * 1024;

function normalizePath(path: string): string {
  return path.replace(/\\/g, '/');
}

function parentDirectory(path: string, fallback: string): string {
  const lastSlash = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
  if (lastSlash <= 0) return fallback;
  return path.slice(0, lastSlash);
}

function fileNameFromPath(path: string): string {
  const normalized = normalizePath(path);
  const idx = normalized.lastIndexOf('/');
  return idx >= 0 ? normalized.slice(idx + 1) : normalized;
}

function extensionFromName(name: string): string {
  const idx = name.lastIndexOf('.');
  if (idx < 0 || idx === name.length - 1) return '';
  return name.slice(idx + 1).toLowerCase();
}

type FileIconKind =
  | 'directory'
  | 'symlink'
  | 'rust'
  | 'typescript'
  | 'javascript'
  | 'json'
  | 'markdown'
  | 'config'
  | 'shell'
  | 'go'
  | 'python'
  | 'image'
  | 'file';

function iconKindForEntry(entry: FileTreeEntry): FileIconKind {
  if (entry.entryType === 'directory') return 'directory';
  if (entry.entryType === 'symlink') return 'symlink';

  const ext = extensionFromName(entry.name);
  if (ext === 'rs') return 'rust';
  if (ext === 'ts' || ext === 'tsx') return 'typescript';
  if (ext === 'js' || ext === 'jsx' || ext === 'mjs' || ext === 'cjs') return 'javascript';
  if (ext === 'json') return 'json';
  if (ext === 'md' || ext === 'mdx') return 'markdown';
  if (ext === 'yml' || ext === 'yaml' || ext === 'toml' || ext === 'ini') return 'config';
  if (ext === 'sh' || ext === 'bash' || ext === 'zsh') return 'shell';
  if (ext === 'go') return 'go';
  if (ext === 'py') return 'python';
  if (ext === 'png' || ext === 'jpg' || ext === 'jpeg' || ext === 'gif' || ext === 'ico' || ext === 'svg') {
    return 'image';
  }
  return 'file';
}

function iconColorForKind(kind: FileIconKind): string {
  if (kind === 'directory') return 'var(--color-marine-400)';
  if (
    kind === 'rust'
    || kind === 'go'
    || kind === 'python'
    || kind === 'typescript'
    || kind === 'javascript'
  ) {
    return 'var(--color-green-400)';
  }
  if (kind === 'markdown' || kind === 'json' || kind === 'config') return 'var(--color-warning-400)';
  if (kind === 'image') return 'var(--color-danger-400)';
  return 'var(--color-text-muted)';
}

function renderTreeIcon(kind: FileIconKind, expanded: boolean): ReactNode {
  switch (kind) {
    case 'directory':
      return expanded ? (
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">
          <path d="M1.5 6.5h13l-1.2 6H2.7L1.5 6.5Z" stroke="currentColor" strokeWidth="1.2" />
          <path d="M1.5 6.5V4.2c0-.9.7-1.7 1.7-1.7h3l1.1 1.2h5.5c1 0 1.7.7 1.7 1.7v1.1" stroke="currentColor" strokeWidth="1.2" />
        </svg>
      ) : (
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">
          <path d="M1.5 4.2c0-.9.7-1.7 1.7-1.7h3l1.1 1.2h5.5c1 0 1.7.7 1.7 1.7v6.4c0 .9-.7 1.7-1.7 1.7H3.2c-.9 0-1.7-.7-1.7-1.7V4.2Z" stroke="currentColor" strokeWidth="1.2" />
        </svg>
      );
    case 'symlink':
      return (
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">
          <path d="M6 10.5H4.7a2.7 2.7 0 0 1 0-5.4H6" stroke="currentColor" strokeWidth="1.2" />
          <path d="M10 5.1h1.3a2.7 2.7 0 1 1 0 5.4H10" stroke="currentColor" strokeWidth="1.2" />
          <path d="M6.4 8h3.2" stroke="currentColor" strokeWidth="1.2" />
        </svg>
      );
    case 'rust':
      return (
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">
          <circle cx="8" cy="8" r="5.5" stroke="currentColor" strokeWidth="1.2" />
          <path d="M8 4.6v6.8M4.6 8h6.8" stroke="currentColor" strokeWidth="1.2" />
        </svg>
      );
    case 'typescript':
    case 'javascript':
    case 'go':
    case 'python':
    case 'shell':
      return (
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">
          <rect x="2.2" y="2.2" width="11.6" height="11.6" rx="2" stroke="currentColor" strokeWidth="1.2" />
          <path d="M5 8h6M8 5v6" stroke="currentColor" strokeWidth="1.2" />
        </svg>
      );
    case 'json':
    case 'config':
    case 'markdown':
      return (
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">
          <path d="M4 1.8h5l3 3v9.4H4V1.8Z" stroke="currentColor" strokeWidth="1.2" />
          <path d="M9 1.8V5h3" stroke="currentColor" strokeWidth="1.2" />
        </svg>
      );
    case 'image':
      return (
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">
          <rect x="1.8" y="2.3" width="12.4" height="11.4" rx="1.8" stroke="currentColor" strokeWidth="1.2" />
          <circle cx="5.6" cy="6" r="1.1" fill="currentColor" />
          <path d="m3.5 11.6 2.6-2.3 2.1 1.7 2.3-2.2 2 2.8" stroke="currentColor" strokeWidth="1.2" />
        </svg>
      );
    case 'file':
    default:
      return (
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">
          <path d="M4 1.8h5l3 3v9.4H4V1.8Z" stroke="currentColor" strokeWidth="1.2" />
          <path d="M9 1.8V5h3" stroke="currentColor" strokeWidth="1.2" />
          <path d="M5.5 8.1h5M5.5 10.2h5" stroke="currentColor" strokeWidth="1.2" />
        </svg>
      );
  }
}

function languageFromPath(path: string): string {
  const name = fileNameFromPath(path).toLowerCase();
  if (name === 'cargo.toml' || name.endsWith('.toml')) return 'toml';
  if (name.endsWith('.rs')) return 'rust';
  if (name.endsWith('.ts')) return 'typescript';
  if (name.endsWith('.tsx')) return 'tsx';
  if (name.endsWith('.js')) return 'javascript';
  if (name.endsWith('.jsx')) return 'jsx';
  if (name.endsWith('.json')) return 'json';
  if (name.endsWith('.md') || name.endsWith('.mdx')) return 'markdown';
  if (name.endsWith('.yml') || name.endsWith('.yaml')) return 'yaml';
  if (name.endsWith('.py')) return 'python';
  if (name.endsWith('.go')) return 'go';
  if (name.endsWith('.sh') || name.endsWith('.bash') || name.endsWith('.zsh')) return 'shell';
  return 'text';
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

  const [selectedFilePath, setSelectedFilePath] = useState<string | null>(null);
  const [preview, setPreview] = useState<FilePreview | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);

  const pollTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pollCursor = useRef(0);
  const debounceTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const previewRequestId = useRef(0);

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
  // Load file preview
  // -------------------------------------------------------------------------
  const loadPreview = useCallback(async (path: string) => {
    const requestId = ++previewRequestId.current;
    setPreviewLoading(true);
    try {
      const result = await readFilePreview(path, DEFAULT_PREVIEW_BYTES);
      if (previewRequestId.current !== requestId) return;
      setPreview(result);
    } catch (err) {
      if (previewRequestId.current !== requestId) return;
      setPreview({
        path,
        content: null,
        truncated: false,
        isBinary: false,
        size: null,
        error: err instanceof Error ? err.message : String(err),
      });
    } finally {
      if (previewRequestId.current === requestId) {
        setPreviewLoading(false);
      }
    }
  }, []);

  const handleSelectFile = useCallback((path: string) => {
    setSelectedFilePath(path);
    loadPreview(path);
  }, [loadPreview]);

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
    if (selectedFilePath) {
      loadPreview(selectedFilePath);
    }
  }, [loadPreview, loadRoot, selectedFilePath]);

  // -------------------------------------------------------------------------
  // Debounced refresh for watcher events
  // -------------------------------------------------------------------------
  const debouncedRefresh = useCallback(
    (events: FileWatchEvent[]) => {
      if (events.length === 0) return;

      const dirsToRefresh = new Set<string>();
      const selectedParent = selectedFilePath
        ? normalizePath(parentDirectory(selectedFilePath, effectivePath))
        : null;
      let refreshSelectedPreview = false;

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

        if (selectedFilePath) {
          const normalizedEvent = normalizePath(evt.path);
          const normalizedSelected = normalizePath(selectedFilePath);
          if (
            normalizedEvent === normalizedSelected
            || (selectedParent !== null && normalizePath(parentDir) === selectedParent)
          ) {
            refreshSelectedPreview = true;
          }
        }
      }

      if (dirsToRefresh.size === 0 && !refreshSelectedPreview) return;

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
        if (refreshSelectedPreview && selectedFilePath) {
          loadPreview(selectedFilePath);
        }
      }, DEBOUNCE_REFRESH_MS);
    },
    [effectivePath, expanded, loadPreview, loadRoot, loadSubdir, selectedFilePath],
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
    setSelectedFilePath(null);
    setPreview(null);
    setPreviewLoading(false);
    previewRequestId.current = 0;
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

  const bodyStyle: CSSProperties = {
    display: 'flex',
    flex: 1,
    minHeight: 0,
    overflow: 'hidden',
  };

  const treePaneStyle: CSSProperties = {
    width: '40%',
    minWidth: 300,
    borderRight: '1px solid var(--color-border-700)',
    display: 'flex',
    flexDirection: 'column',
    minHeight: 0,
    overflow: 'hidden',
  };

  const treeAreaStyle: CSSProperties = {
    flex: 1,
    overflowY: 'auto',
    padding: 'var(--space-2)',
    backgroundColor: 'var(--color-bg-950)',
    fontFamily: 'var(--font-mono)',
    fontSize: 'var(--text-xs)',
  };

  const previewPaneStyle: CSSProperties = {
    flex: 1,
    minWidth: 0,
    minHeight: 0,
    display: 'flex',
    flexDirection: 'column',
    backgroundColor: 'var(--color-bg-950)',
  };

  const previewHeaderStyle: CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    gap: 'var(--space-2)',
    padding: 'var(--space-2) var(--space-3)',
    borderBottom: '1px solid var(--color-border-700)',
    backgroundColor: 'var(--color-bg-900)',
    minHeight: 37,
    flexShrink: 0,
  };

  const previewNameStyle: CSSProperties = {
    fontFamily: 'var(--font-mono)',
    fontSize: 'var(--text-xs)',
    color: 'var(--color-text-secondary)',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  };

  const previewBodyStyle: CSSProperties = {
    flex: 1,
    minHeight: 0,
    overflow: 'auto',
  };

  const previewCodeStyle: CSSProperties = {
    margin: 0,
    padding: 'var(--space-3)',
    minHeight: '100%',
    fontFamily: 'var(--font-mono)',
    fontSize: '12px',
    lineHeight: 1.5,
    color: 'var(--color-text-secondary)',
    whiteSpace: 'pre',
    overflowX: 'auto',
  };

  const selectedFileName = selectedFilePath ? fileNameFromPath(selectedFilePath) : null;

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

      <div style={bodyStyle}>
        <div style={treePaneStyle}>
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
                selectedFilePath={selectedFilePath}
                onToggle={toggleDir}
                onSelectFile={handleSelectFile}
              />
            ))}
          </div>
        </div>

        <div style={previewPaneStyle} data-testid="file-preview-pane">
          <div style={previewHeaderStyle}>
            <span style={previewNameStyle} data-testid="file-preview-name">
              {selectedFileName ?? 'Select a file to preview'}
            </span>
            {selectedFilePath && (
              <span
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: '10px',
                  color: 'var(--color-text-muted)',
                  border: '1px solid var(--color-border-700)',
                  borderRadius: 'var(--radius-sm)',
                  padding: '1px 6px',
                  flexShrink: 0,
                }}
                data-testid="file-preview-language"
              >
                {languageFromPath(selectedFilePath)}
              </span>
            )}
          </div>

          <div style={previewBodyStyle}>
            {!selectedFilePath && (
              <div
                style={{
                  color: 'var(--color-text-muted)',
                  fontSize: 'var(--text-xs)',
                  padding: 'var(--space-4)',
                  textAlign: 'center',
                }}
                data-testid="file-preview-empty"
              >
                Select a file in the tree to preview text/code content.
              </div>
            )}

            {selectedFilePath && previewLoading && (
              <div
                style={{
                  color: 'var(--color-text-muted)',
                  fontSize: 'var(--text-xs)',
                  padding: 'var(--space-4)',
                }}
                data-testid="file-preview-loading"
              >
                Loading preview...
              </div>
            )}

            {selectedFilePath && !previewLoading && preview?.error && (
              <div
                style={{
                  color: 'var(--color-danger-400)',
                  fontSize: 'var(--text-xs)',
                  padding: 'var(--space-4)',
                }}
                data-testid="file-preview-error"
              >
                {preview.error}
              </div>
            )}

            {selectedFilePath && !previewLoading && preview && !preview.error && preview.isBinary && (
              <div
                style={{
                  color: 'var(--color-warning-400)',
                  fontSize: 'var(--text-xs)',
                  padding: 'var(--space-4)',
                }}
                data-testid="file-preview-binary"
              >
                Binary file preview is not available.{preview.size !== null ? ` (${formatSize(preview.size)})` : ''}
              </div>
            )}

            {selectedFilePath && !previewLoading && preview && !preview.error && !preview.isBinary && (
              <>
                <pre style={previewCodeStyle} data-testid="file-preview-content">
                  {preview.content ?? ''}
                </pre>
                {preview.truncated && (
                  <div
                    style={{
                      padding: 'var(--space-2) var(--space-3)',
                      borderTop: '1px solid var(--color-border-700)',
                      color: 'var(--color-warning-400)',
                      fontSize: 'var(--text-xs)',
                    }}
                    data-testid="file-preview-truncated"
                  >
                    Preview truncated to {formatSize(DEFAULT_PREVIEW_BYTES)}.
                  </div>
                )}
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function TreeNode({
  entry,
  depth,
  expanded,
  selectedFilePath,
  onToggle,
  onSelectFile,
}: {
  entry: FileTreeEntry;
  depth: number;
  expanded: Map<string, FileTreeEntry[]>;
  selectedFilePath: string | null;
  onToggle: (path: string) => void;
  onSelectFile: (path: string) => void;
}) {
  const isDir = entry.entryType === 'directory';
  const isExpanded = expanded.has(entry.path);
  const children = isExpanded ? expanded.get(entry.path) ?? [] : [];
  const iconKind = iconKindForEntry(entry);
  const isSelected = !isDir
    && selectedFilePath !== null
    && normalizePath(selectedFilePath) === normalizePath(entry.path);

  const nodeStyle: CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    gap: 'var(--space-1)',
    padding: '2px var(--space-1)',
    paddingLeft: `calc(var(--space-3) * ${depth} + var(--space-1))`,
    cursor: 'pointer',
    color: isDir ? 'var(--color-text-primary)' : 'var(--color-text-secondary)',
    borderRadius: 'var(--radius-sm)',
    backgroundColor: isSelected
      ? 'color-mix(in srgb, var(--color-marine-500) 12%, transparent)'
      : 'transparent',
  };

  const chevronStyle: CSSProperties = {
    flexShrink: 0,
    width: 12,
    textAlign: 'center',
    color: isDir
      ? isExpanded
        ? 'var(--color-marine-400)'
        : 'var(--color-text-muted)'
      : 'transparent',
  };

  const iconStyle: CSSProperties = {
    flexShrink: 0,
    width: 16,
    height: 16,
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    textAlign: 'center',
    color: iconColorForKind(iconKind),
  };

  const handleClick = () => {
    if (isDir) {
      onToggle(entry.path);
      return;
    }
    onSelectFile(entry.path);
  };

  return (
    <>
      <div
        style={nodeStyle}
        onClick={handleClick}
        data-testid={`tree-node-${entry.name}`}
      >
        <span style={chevronStyle}>
          {isDir ? (isExpanded ? '▾' : '▸') : ' '}
        </span>
        <span
          style={iconStyle}
          data-testid={`tree-icon-${entry.name}`}
          title={`icon-${iconKind}`}
        >
          {renderTreeIcon(iconKind, isExpanded)}
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
      {isExpanded
        && children.map((child) => (
          <TreeNode
            key={child.path}
            entry={child}
            depth={depth + 1}
            expanded={expanded}
            selectedFilePath={selectedFilePath}
            onToggle={onToggle}
            onSelectFile={onSelectFile}
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
