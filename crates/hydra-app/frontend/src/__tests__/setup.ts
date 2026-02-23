import '@testing-library/jest-dom/vitest';

// React 19 act() environment configuration
(globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
