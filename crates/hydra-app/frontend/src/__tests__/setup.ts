import '@testing-library/jest-dom/vitest';
import { vi } from 'vitest';

// React 19 act() environment configuration
(globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;

// ---------------------------------------------------------------------------
// ResizeObserver stub (not available in jsdom)
// ---------------------------------------------------------------------------
if (typeof globalThis.ResizeObserver === 'undefined') {
  globalThis.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  } as unknown as typeof globalThis.ResizeObserver;
}

// ---------------------------------------------------------------------------
// xterm.js mock for jsdom test environment (P4.9.3)
//
// xterm.js requires a real DOM canvas and GPU context that jsdom does not
// provide.  This lightweight mock:
//   - tracks raw write() calls (including ANSI) on a per-instance array
//   - renders stripped-ANSI plain text into a DOM <div> so that
//     @testing-library/react `getByText` queries continue to work
//   - stubs lifecycle methods (open, dispose, clear, reset, loadAddon)
//
// Tests that need to assert on raw ANSI preservation can access the mock's
// `__rawWrites` via the globally-exposed `__xtermInstances` array.
// ---------------------------------------------------------------------------

function lineStart(text: string, cursor: number): number {
  const idx = text.lastIndexOf('\n', Math.max(0, cursor - 1));
  return idx >= 0 ? idx + 1 : 0;
}

function lineEnd(text: string, cursor: number): number {
  const idx = text.indexOf('\n', cursor);
  return idx >= 0 ? idx : text.length;
}

function applyAnsiToText(
  existing: string,
  cursorStart: number,
  input: string,
): { text: string; cursor: number } {
  let text = existing;
  let cursor = Math.max(0, Math.min(cursorStart, text.length));
  let i = 0;

  while (i < input.length) {
    const ch = input[i];

    if (ch === '\u001b') {
      // Skip OSC sequences.
      if (input[i + 1] === ']') {
        const bellIdx = input.indexOf('\u0007', i + 2);
        const stIdx = input.indexOf('\u001b\\', i + 2);
        if (bellIdx >= 0) {
          i = bellIdx + 1;
          continue;
        }
        if (stIdx >= 0) {
          i = stIdx + 2;
          continue;
        }
        break;
      }

      // Handle CSI sequences.
      if (input[i + 1] === '[') {
        let j = i + 2;
        while (j < input.length && !/[A-Za-z]/.test(input[j])) j++;
        if (j >= input.length) break;
        const final = input[j];
        const params = input.slice(i + 2, j);

        switch (final) {
          case 'J':
            // ED: ESC[2J clear entire screen.
            if (params === '2') {
              text = '';
              cursor = 0;
            }
            break;
          case 'K': {
            // EL: clear to end of line.
            const end = lineEnd(text, cursor);
            text = text.slice(0, cursor) + text.slice(end);
            break;
          }
          case 'H':
          case 'f':
            // CUP/HVP: only model home for tests.
            if (params === '' || params === '1;1') {
              cursor = 0;
            }
            break;
          case 'm':
          case 'A':
          case 'B':
          case 'C':
          case 'D':
            // SGR and cursor moves are preserved in raw writes; no-op in mock text.
            break;
          default:
            break;
        }

        i = j + 1;
        continue;
      }
    }

    if (ch === '\r') {
      cursor = lineStart(text, cursor);
      i += 1;
      continue;
    }

    if (ch === '\n') {
      if (cursor >= text.length) {
        text += '\n';
      } else {
        text = text.slice(0, cursor) + '\n' + text.slice(cursor);
      }
      cursor += 1;
      i += 1;
      continue;
    }

    // Drop non-printable controls.
    if (ch < ' ' || ch === '\x7f') {
      i += 1;
      continue;
    }

    if (cursor < text.length) {
      text = text.slice(0, cursor) + ch + text.slice(cursor + 1);
    } else {
      text += ch;
    }
    cursor += 1;
    i += 1;
  }

  return { text, cursor };
}

export interface MockTerminalInstance {
  __rawWrites: string[];
  __element: HTMLDivElement | null;
  options: Record<string, unknown>;
}

// Global list of all MockTerminal instances created during a test.
// Tests can inspect (globalThis as any).__xtermInstances.
const instances: MockTerminalInstance[] = [];
(globalThis as Record<string, unknown>).__xtermInstances = instances;

class MockTerminal implements MockTerminalInstance {
  __rawWrites: string[] = [];
  __element: HTMLDivElement | null = null;
  options: Record<string, unknown> = {};
  __cursor = 0;

  constructor(opts?: Record<string, unknown>) {
    if (opts) this.options = { ...opts };
    instances.push(this);
  }

  open(parent: HTMLElement) {
    this.__element = document.createElement('div');
    this.__element.setAttribute('data-testid', 'xterm-screen');
    parent.appendChild(this.__element);
  }

  write(data: string | Uint8Array, callback?: () => void) {
    const str = typeof data === 'string' ? data : new TextDecoder().decode(data);
    this.__rawWrites.push(str);
    if (this.__element) {
      const next = applyAnsiToText(this.__element.textContent ?? '', this.__cursor, str);
      this.__cursor = next.cursor;
      this.__element.textContent = next.text;
    }
    if (callback) callback();
  }

  loadAddon() {}

  clear() {
    this.__rawWrites = [];
    this.__cursor = 0;
    if (this.__element) this.__element.textContent = '';
  }

  reset() {
    this.__cursor = 0;
  }

  dispose() {
    if (this.__element?.parentNode) {
      this.__element.parentNode.removeChild(this.__element);
    }
    this.__element = null;
  }
}

vi.mock('@xterm/xterm', () => ({ Terminal: MockTerminal }));

vi.mock('@xterm/addon-fit', () => ({
  FitAddon: class {
    fit() {}
    dispose() {}
  },
}));

// Clean up instance tracking between tests
afterEach(() => {
  instances.length = 0;
});
