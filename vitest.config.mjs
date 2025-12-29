import react from '@vitejs/plugin-react';
import path from 'node:path';
import { defineConfig } from 'vitest/config';

const srcRoot = path.resolve(__dirname, 'app/AideonDesktop/src');
const testsRoot = path.resolve(__dirname, 'app/AideonDesktop/tests');

const praxisRoot = path.join(srcRoot, 'workspaces/praxis');

export default defineConfig({
  resolve: {
    alias: [
      { find: '@', replacement: srcRoot },
      { find: 'aideon', replacement: path.join(srcRoot, 'aideon') },
      { find: 'praxis', replacement: praxisRoot },
      {
        find: 'design-system/reactflow',
        replacement: path.join(srcRoot, 'design-system/components'),
      },
      { find: 'design-system', replacement: path.join(srcRoot, 'design-system') },
      { find: 'adapters', replacement: path.join(srcRoot, 'adapters') },
      { find: 'dtos', replacement: path.join(srcRoot, 'dtos') },
      { find: 'lib', replacement: path.join(srcRoot, 'lib') },
      {
        find: '@tauri-apps/api/core',
        replacement: path.join(testsRoot, 'adapters/stubs/tauri-core.ts'),
      },
    ],
  },
  plugins: [react()],
  test: {
    environment: 'jsdom',
    setupFiles: [path.resolve(__dirname, 'tests/setup.ts')],
    coverage: {
      provider: 'istanbul',
      reporter: ['text', 'lcov', 'html'],
      reportOnFailure: true,
      thresholds: {
        lines: 0.8,
        functions: 0.8,
        branches: 0.8,
        statements: 0.8,
      },
      include: ['app/AideonDesktop/src/**/*.{ts,tsx}'],
      exclude: [
        '**/*.d.ts',
        '**/*.test.*',
        'app/**/dist/**',
        'scripts/**',
        'app/AideonDesktop/src/design-system/components/**',
        'app/AideonDesktop/src/types/**',
      ],
    },
  },
});
