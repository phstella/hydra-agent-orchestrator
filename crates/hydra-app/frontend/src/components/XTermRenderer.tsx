import { useEffect, useRef, useImperativeHandle, forwardRef } from 'react';
import type { CSSProperties } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';

/**
 * Hydra terminal theme derived from design tokens (tokens.css).
 * Hard-coded hex values are required here because xterm.js does not
 * support CSS custom-property references in its ITheme interface.
 */
const HYDRA_THEME = {
  background: '#060B0A',   // --color-bg-950
  foreground: '#A7C4B8',   // --color-text-secondary
  cursor: '#A7C4B8',       // --color-text-secondary
  cursorAccent: '#060B0A', // --color-bg-950
  selectionBackground: '#2F6F9F80', // --color-marine-500 @ 50%
  selectionForeground: '#F0FDF4',   // --color-text-primary
  black: '#060B0A',
  red: '#EF4444',          // --color-danger-500
  green: '#22C55E',        // --color-green-500
  yellow: '#EAB308',       // --color-warning-500
  blue: '#4C8DBF',         // --color-marine-400
  magenta: '#A78BFA',
  cyan: '#7AB3D4',         // --color-marine-300
  white: '#F0FDF4',        // --color-text-primary
  brightBlack: '#6B8F80',  // --color-text-muted
  brightRed: '#F87171',    // --color-danger-400
  brightGreen: '#4ADE80',  // --color-green-400
  brightYellow: '#FACC15', // --color-warning-400
  brightBlue: '#7AB3D4',   // --color-marine-300
  brightMagenta: '#C4B5FD',
  brightCyan: '#A5F3FC',
  brightWhite: '#FFFFFF',
} as const;

const SCROLLBACK_LINES = 10_000;
const WRITE_CHUNK_CHARS_BASE = 64 * 1024;
const WRITE_CHUNK_CHARS_BURST = 256 * 1024;
const FRAME_BUDGET_MS_BASE = 4;
const FRAME_BUDGET_MS_BURST = 10;
const BACKLOG_SOFT_LIMIT_CHARS = 500_000;
const BACKLOG_HARD_LIMIT_CHARS = 2_000_000;
const BACKLOG_RETAIN_CHARS = 1_000_000;

type DisposableAddon = { dispose?: () => void };

function nowMs(): number {
  if (typeof globalThis.performance?.now === 'function') {
    return globalThis.performance.now();
  }
  return Date.now();
}

function importOptionalWebglAddon(): Promise<{ WebglAddon?: new () => DisposableAddon }> {
  const dynamicImport = new Function('specifier', 'return import(specifier)') as (
    specifier: string,
  ) => Promise<{ WebglAddon?: new () => DisposableAddon }>;
  return dynamicImport('@xterm/addon-webgl');
}

function scheduleFrame(cb: () => void): number {
  if (typeof globalThis.requestAnimationFrame === 'function') {
    return globalThis.requestAnimationFrame(cb);
  }
  return globalThis.setTimeout(cb, 0);
}

function cancelFrame(id: number): void {
  if (typeof globalThis.cancelAnimationFrame === 'function') {
    globalThis.cancelAnimationFrame(id);
    return;
  }
  globalThis.clearTimeout(id);
}

export interface XTermRendererProps {
  /**
   * Key that triggers a full terminal reset when it changes.
   * Typically the sessionId — switching sessions clears the buffer.
   */
  resetKey: string | null;

  /**
   * Raw text chunks to write, in order.  The renderer tracks how many
   * chunks have been written and only appends new ones on each render.
   * If the array shrinks (e.g. bounded-buffer eviction), the terminal
   * resets and replays the current array.
   */
  chunks: string[];

  /**
   * P4.9.5: Callback for terminal keyboard input. When set, stdin is
   * enabled and keystrokes are forwarded to this handler.
   */
  onData?: (data: string) => void;

  /**
   * Reports fitted terminal dimensions so PTY rows/cols can be resized
   * to match the visible xterm viewport.
   */
  onResize?: (cols: number, rows: number) => void;
}

export interface XTermRendererHandle {
  focus: () => void;
  appendChunk: (chunk: string) => void;
  replaceChunks: (chunks: string[]) => void;
}

export const XTermRenderer = forwardRef<XTermRendererHandle, XTermRendererProps>(
  function XTermRenderer({ resetKey, chunks, onData, onResize }, ref) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const webglAddonRef = useRef<DisposableAddon | null>(null);
  const dataListenerRef = useRef<{ dispose: () => void } | null>(null);
  const onResizeRef = useRef<typeof onResize>(onResize);
  const pendingBufferRef = useRef('');
  const droppedCharsRef = useRef(0);
  const flushFrameRef = useRef<number | null>(null);
  const writtenRef = useRef(0);
  const resetKeyRef = useRef<string | null>(null);
  const firstChunkRef = useRef<string | null>(null);
  const lastChunkRef = useRef<string | null>(null);
  const lastSizeRef = useRef<{ cols: number; rows: number } | null>(null);

  const resetTerminal = () => {
    const term = termRef.current;
    if (!term) return;
    term.clear();
    term.reset();
    term.options.theme = HYDRA_THEME;
    writtenRef.current = 0;
    firstChunkRef.current = null;
    lastChunkRef.current = null;
    lastSizeRef.current = null;
    pendingBufferRef.current = '';
    droppedCharsRef.current = 0;
    if (flushFrameRef.current !== null) {
      cancelFrame(flushFrameRef.current);
      flushFrameRef.current = null;
    }
  };

  const emitResize = () => {
    const term = termRef.current as (Terminal & { cols?: number; rows?: number }) | null;
    const cb = onResizeRef.current;
    if (!term || typeof cb !== 'function') return;
    if (typeof term.cols !== 'number' || typeof term.rows !== 'number') return;
    if (term.cols <= 0 || term.rows <= 0) return;
    const prev = lastSizeRef.current;
    if (prev && prev.cols === term.cols && prev.rows === term.rows) return;
    lastSizeRef.current = { cols: term.cols, rows: term.rows };
    cb(term.cols, term.rows);
  };

  function trimPendingBacklogIfNeeded() {
    const currentSize = pendingBufferRef.current.length;
    if (currentSize <= BACKLOG_HARD_LIMIT_CHARS) return;
    const dropChars = currentSize - BACKLOG_RETAIN_CHARS;
    pendingBufferRef.current = pendingBufferRef.current.slice(dropChars);
    droppedCharsRef.current += dropChars;
  }

  function scheduleFlush() {
    if (flushFrameRef.current !== null) return;
    flushFrameRef.current = scheduleFrame(flushPendingWrites);
  }

  function flushPendingWrites() {
    flushFrameRef.current = null;
    const term = termRef.current;
    if (!term) {
      pendingBufferRef.current = '';
      droppedCharsRef.current = 0;
      return;
    }

    if (pendingBufferRef.current.length === 0) {
      if (droppedCharsRef.current > 0) {
        term.write(
          `\r\n[hydra] dropped ${droppedCharsRef.current.toLocaleString()} chars to keep terminal responsive\r\n`,
        );
        droppedCharsRef.current = 0;
      }
      return;
    }

    const backlogChars = pendingBufferRef.current.length;
    const burstMode = backlogChars > BACKLOG_SOFT_LIMIT_CHARS;
    const maxChunk = burstMode ? WRITE_CHUNK_CHARS_BURST : WRITE_CHUNK_CHARS_BASE;
    const deadline = nowMs() + (burstMode ? FRAME_BUDGET_MS_BURST : FRAME_BUDGET_MS_BASE);

    while (pendingBufferRef.current.length > 0 && nowMs() < deadline) {
      const chunk = pendingBufferRef.current.slice(0, maxChunk);
      pendingBufferRef.current = pendingBufferRef.current.slice(chunk.length);
      term.write(chunk);
    }

    if (pendingBufferRef.current.length > 0) {
      scheduleFlush();
      return;
    }

    if (droppedCharsRef.current > 0) {
      term.write(
        `\r\n[hydra] dropped ${droppedCharsRef.current.toLocaleString()} chars to keep terminal responsive\r\n`,
      );
      droppedCharsRef.current = 0;
    }
  }

  useImperativeHandle(ref, () => ({
    focus: () => {
      if (typeof termRef.current?.focus === 'function') {
        termRef.current.focus();
      }
    },
    appendChunk: (chunk: string) => {
      if (!chunk) return;
      pendingBufferRef.current += chunk;
      trimPendingBacklogIfNeeded();
      scheduleFlush();
    },
    replaceChunks: (nextChunks: string[]) => {
      resetTerminal();
      if (nextChunks.length === 0) return;
      pendingBufferRef.current = nextChunks.join('');
      trimPendingBacklogIfNeeded();
      writtenRef.current = nextChunks.length;
      firstChunkRef.current = nextChunks[0] ?? null;
      lastChunkRef.current = nextChunks[nextChunks.length - 1] ?? null;
      scheduleFlush();
    },
  }));

  // ── Mount / unmount terminal ──────────────────────────────────────────
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const stdinEnabled = !!onData;

    const term = new Terminal({
      cursorBlink: stdinEnabled,
      cursorStyle: stdinEnabled ? 'block' : 'bar',
      cursorInactiveStyle: stdinEnabled ? 'outline' : 'none',
      disableStdin: !stdinEnabled,
      scrollback: SCROLLBACK_LINES,
      fontSize: 13,
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
      theme: HYDRA_THEME,
      convertEol: false,
      allowTransparency: false,
    });

    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(el);

    termRef.current = term;
    fitRef.current = fit;
    writtenRef.current = 0;
    resetKeyRef.current = resetKey;
    firstChunkRef.current = null;
    lastChunkRef.current = null;
    lastSizeRef.current = null;

    // Initial fit after open
    try { fit.fit(); } catch { /* layout not ready */ }
    emitResize();

    // Optional GPU acceleration path. If unavailable, terminal keeps canvas renderer.
    importOptionalWebglAddon()
      .then((mod) => {
        const currentTerm = termRef.current;
        if (!currentTerm || currentTerm !== term || webglAddonRef.current) return;
        const WebglAddonCtor = mod.WebglAddon;
        if (typeof WebglAddonCtor !== 'function') return;
        try {
          const addon = new WebglAddonCtor();
          currentTerm.loadAddon(addon as never);
          webglAddonRef.current = addon;
        } catch {
          // Fallback is intentional; do not surface UI error for renderer fallback.
        }
      })
      .catch(() => {
        // Optional dependency may be absent in some runtimes.
      });

    // Observe container resize
    const ro = new ResizeObserver(() => {
      try { fit.fit(); } catch { /* ignore during transitions */ }
      emitResize();
    });
    ro.observe(el);

    return () => {
      ro.disconnect();
      dataListenerRef.current?.dispose();
      dataListenerRef.current = null;
      if (flushFrameRef.current !== null) {
        cancelFrame(flushFrameRef.current);
        flushFrameRef.current = null;
      }
      pendingBufferRef.current = '';
      droppedCharsRef.current = 0;
      webglAddonRef.current?.dispose?.();
      webglAddonRef.current = null;
      term.dispose();
      termRef.current = null;
      fitRef.current = null;
    };
    // Only re-create if the DOM element changes (never in practice)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Keep resize callback current without re-creating terminal.
  useEffect(() => {
    onResizeRef.current = onResize;
    emitResize();
  }, [onResize]);

  // Keep stdin + keyboard listener in sync with lane/status changes.
  useEffect(() => {
    const term = termRef.current;
    if (!term) return;

    const stdinEnabled = typeof onData === 'function';
    term.options.disableStdin = !stdinEnabled;
    term.options.cursorBlink = stdinEnabled;
    term.options.cursorStyle = stdinEnabled ? 'block' : 'bar';
    term.options.cursorInactiveStyle = stdinEnabled ? 'outline' : 'none';

    dataListenerRef.current?.dispose();
    dataListenerRef.current = null;

    if (stdinEnabled && typeof term.onData === 'function') {
      dataListenerRef.current = term.onData((data) => {
        onData(data);
      });
    }

    return () => {
      dataListenerRef.current?.dispose();
      dataListenerRef.current = null;
    };
  }, [onData]);

  // ── Handle session reset ──────────────────────────────────────────────
  useEffect(() => {
    if (resetKeyRef.current === resetKey) return;
    resetKeyRef.current = resetKey;
    resetTerminal();
    emitResize();
  }, [resetKey]);

  // ── Write new chunks ──────────────────────────────────────────────────
  useEffect(() => {
    if (!termRef.current) return;

    // If chunks array shrank (bounded eviction), replay from scratch
    const replacedWithoutLengthChange = chunks.length === writtenRef.current
      && chunks.length > 0
      && (chunks[0] !== firstChunkRef.current || chunks[chunks.length - 1] !== lastChunkRef.current);
    if (chunks.length < writtenRef.current || replacedWithoutLengthChange) {
      resetTerminal();
    }

    const start = writtenRef.current;
    if (start < chunks.length) {
      pendingBufferRef.current += chunks.slice(start).join('');
      trimPendingBacklogIfNeeded();
      writtenRef.current = chunks.length;
      firstChunkRef.current = chunks[0] ?? null;
      lastChunkRef.current = chunks[chunks.length - 1] ?? null;
      scheduleFlush();
      return;
    }
    if (chunks.length === 0) {
      firstChunkRef.current = null;
      lastChunkRef.current = null;
    }
  }, [chunks]);

  const style: CSSProperties = {
    flex: 1,
    minHeight: 0,
    overflow: 'hidden',
  };

  return (
    <div
      ref={containerRef}
      style={style}
      data-testid="xterm-container"
    />
  );
});
