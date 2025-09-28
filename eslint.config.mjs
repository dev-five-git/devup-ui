import { configs } from 'eslint-plugin-devup'
import jsonc from 'eslint-plugin-jsonc'
import * as mdx from 'eslint-plugin-mdx'
import globals from 'globals'

export default [
  // eslint-plugin-devup
  ...configs.recommended,
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
      'react/no-children-prop': 'off',
    },
  },
  // md, mdx rules
  {
    ...mdx.flat,
    files: ['**/*.{md,mdx}'],
    processor: mdx.createRemarkProcessor({
      lintCodeBlocks: true,
    }),
  },
  // md, mdx code blocks rules
  {
    ...mdx.flatCodeBlocks,
    files: ['**/*.{md,mdx}/*.{js,jsx,ts,tsx}'],
    rules: {
      ...mdx.flatCodeBlocks.rules,
      'react/jsx-no-undef': 'off',
      'react/jsx-tag-spacing': ['error', { beforeClosing: 'never' }],
    },
  },
]
