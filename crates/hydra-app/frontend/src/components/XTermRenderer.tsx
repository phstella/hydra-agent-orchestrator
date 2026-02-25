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
}

export interface XTermRendererHandle {
  focus: () => void;
}

export const XTermRenderer = forwardRef<XTermRendererHandle, XTermRendererProps>(
  function XTermRenderer({ resetKey, chunks, onData }, ref) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const dataListenerRef = useRef<{ dispose: () => void } | null>(null);
  const writtenRef = useRef(0);
  const resetKeyRef = useRef<string | null>(null);

  useImperativeHandle(ref, () => ({
    focus: () => {
      if (typeof termRef.current?.focus === 'function') {
        termRef.current.focus();
      }
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
      convertEol: true,
      allowTransparency: false,
    });

    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(el);

    // Initial fit after open
    try { fit.fit(); } catch { /* layout not ready */ }

    termRef.current = term;
    fitRef.current = fit;
    writtenRef.current = 0;
    resetKeyRef.current = resetKey;

    // Observe container resize
    const ro = new ResizeObserver(() => {
      try { fit.fit(); } catch { /* ignore during transitions */ }
    });
    ro.observe(el);

    return () => {
      ro.disconnect();
      dataListenerRef.current?.dispose();
      dataListenerRef.current = null;
      term.dispose();
      termRef.current = null;
      fitRef.current = null;
    };
    // Only re-create if the DOM element changes (never in practice)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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
    const term = termRef.current;
    if (!term) return;
    term.clear();
    term.reset();
    // Re-apply theme after reset
    term.options.theme = HYDRA_THEME;
    writtenRef.current = 0;
  }, [resetKey]);

  // ── Write new chunks ──────────────────────────────────────────────────
  useEffect(() => {
    const term = termRef.current;
    if (!term) return;

    // If chunks array shrank (bounded eviction), replay from scratch
    if (chunks.length < writtenRef.current) {
      term.clear();
      term.reset();
      term.options.theme = HYDRA_THEME;
      writtenRef.current = 0;
    }

    const start = writtenRef.current;
    if (start >= chunks.length) return;

    for (let i = start; i < chunks.length; i++) {
      term.write(chunks[i]);
    }
    writtenRef.current = chunks.length;
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
