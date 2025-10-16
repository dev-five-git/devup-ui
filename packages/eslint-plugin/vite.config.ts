import dts from 'vite-plugin-dts'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    globals: true,
    coverage: {
      all: true,
      thresholds: {
        '100': true,
      },
    },
  },
  plugins: [
    dts({
      entryRoot: 'src',
      staticImport: true,
      pathsToAliases: false,
      exclude: [
        '**/__tests__/**/*',
        '**/*.test.(tsx|ts|js|jsx)',
        '**/*.test-d.(tsx|ts|js|jsx)',
        'vite.config.ts',
      ],
      include: ['**/src/**/*.ts'],
      copyDtsFiles: true,
      compilerOptions: {
        isolatedModules: false,
        declaration: true,
      },
    }) as any,
  ],
  build: {
    rollupOptions: {
      onwarn: (warning) => {
        if (warning.code === 'MODULE_LEVEL_DIRECTIVE') {
          return
        }
      },
      external: (source) => {
        return !(source.includes('src') || source.startsWith('.'))
      },

      output: {
        dir: 'dist',
        preserveModules: true,
        preserveModulesRoot: 'src',

        exports: 'named',
        assetFileNames({ name }) {
          return name?.replace(/^src\//g, '') ?? ''
        },
      },
    },
    lib: {
      formats: ['es', 'cjs'],
      entry: {
        index: 'src/index.ts',
      },
    },
    outDir: 'dist',
  },
})
