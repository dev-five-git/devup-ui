import { DevupUI } from '@devup-ui/vite-plugin'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'
import dts from 'vite-plugin-dts'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    react(),
    DevupUI({
      extractCss: false,
    }),
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
      include: ['**/src/**/*.(ts|tsx)', 'df/*.d.ts'],
      copyDtsFiles: true,
      compilerOptions: {
        isolatedModules: false,
        declaration: true,
      },
    }),
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
        index: 'src/index.tsx',
      },
    },
    outDir: 'dist',
  },
})
