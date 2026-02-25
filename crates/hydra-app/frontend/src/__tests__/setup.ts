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

function stripAnsi(str: string): string {
  return str
    .replace(/\u001b\][^\u0007]*(?:\u0007|\u001b\\)/g, '')
    .replace(/\u001b\[[0-9;?]*[ -/]*[@-~]/g, '')
    .replace(/\r\n/g, '\n')
    .replace(/\r/g, '')
    .replace(/[\x00-\x08\x0B-\x1F\x7F]/g, '');
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
      const plain = stripAnsi(str);
      this.__element.textContent = (this.__element.textContent ?? '') + plain;
    }
    if (callback) callback();
  }

  loadAddon() {}

  clear() {
    this.__rawWrites = [];
    if (this.__element) this.__element.textContent = '';
  }

  reset() {}

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
