import devupUi from '@devup-ui/eslint-plugin'
import devup from 'eslint-plugin-devup'
import eslintPlugin from 'eslint-plugin-eslint-plugin'
import jsonc from 'eslint-plugin-jsonc'
import globals from 'globals'

export default [
  {
    ignores: [
      'benchmark/next-panda-css/styled-system',
      'bindings/devup-ui-wasm/pkg',
      'test-results',
      'release-results.json',
      'release-package-build.json',
    ],
  },
  // eslint-plugin-devup
  ...devup.configs.recommended.filter(
    (config) => !('plugins' in config && '@devup-ui' in config.plugins),
  ),
  // eslint-plugin-jsonc
  ...jsonc.configs['flat/recommended-with-json'],
  ...jsonc.configs['flat/recommended-with-jsonc'],
  // globals (node, browser, builtin)
  {
    files: ['**/*.{js,mjs,cjs}'],
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
        ...globals.builtin,
      },
    },
    rules: {
      // js require import allowed
      '@typescript-eslint/no-require-imports': 'off',
    },
  },
  // benchmark no console rules
  {
    files: ['benchmark.js'],
    rules: {
      'no-console': [
        'error',
        {
          allow: ['info', 'debug', 'warn', 'error', 'profile', 'profileEnd'],
        },
      ],
    },
  },
  // create-style-context.mjs no children prop
  {
    files: ['**/*.mjs'],
    rules: {
      '@eslint-react/jsx-no-children-prop': 'off',
    },
  },
  // eslint-plugin rule
  {
    ...eslintPlugin.configs.recommended,
    files: ['packages/eslint-plugin/**/*.{js,jsx,ts,tsx}'],
  },
  {
    ignores: ['**/*.md'],
  },
  ...devupUi.configs.recommended,
  // Published APIs and React 18 fixtures must not migrate to React 19-only APIs.
  // https://eslint-react.xyz/docs/configuration/configure-analyzer#version
  {
    files: [
      'packages/react/**',
      'packages/components/**',
      'apps/react-18/**',
      // These examples document supported input for the dual-major library.
      'apps/landing/src/app/(detail)/docs/migration/styled-components/page.mdx/**',
    ],
    settings: {
      'react-x': {
        version: '18.0.0',
      },
    },
  },
  // the snippets of this page are .css.ts stylesheets, which the build runs as they are
  {
    files: ['apps/landing/src/app/(detail)/docs/migration/vanilla-extract/**'],
    rules: {
      '@devup-ui/css-utils-literal-only': 'off',
    },
  },
]
