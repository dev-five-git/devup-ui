import preserveDirectives from 'rollup-plugin-preserve-directives'
import { defineConfig } from 'vite'
import dts from 'vite-plugin-dts'

export default defineConfig({
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
      include: ['**/src/**/*.ts', '**/src/**/*.tsx'],
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
      plugins: [preserveDirectives()],
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
