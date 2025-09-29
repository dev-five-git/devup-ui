import { noUselessTailingNulls } from 'src/rules/no-useless-tailing-nulls'

export default [
  {
    ignores: [
      '**/node_modules/',
      '**/build/',
      '**/__snapshots__/',
      '!**/src/**',
      '!vite.config.ts',
      '!**/.storybook/**',
      '**/storybook-static/',
      '**/dist/',
      '**/next-env.d.ts',
      '**/out/',
      '**/.next/',
      '**/public/',
      '**/.df/',
    ],
  },
  {
    plugins: {
      '@devup-ui': {
        rules: {
          'no-useless-tailing-nulls': noUselessTailingNulls,
        },
      },
    },
    rules: {
      '@devup-ui/no-useless-tailing-nulls': 'error',
    },
  },
]
