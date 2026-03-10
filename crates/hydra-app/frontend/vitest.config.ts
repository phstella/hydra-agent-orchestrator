/// <reference types="vitest" />
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    include: ['src/**/__tests__/**/*.test.{ts,tsx}'],
    setupFiles: ['src/__tests__/setup.ts'],
  },
});
