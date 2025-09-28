import tsParser from '@typescript-eslint/parser'
import { configs } from 'eslint-plugin-devup'
import jsonc from 'eslint-plugin-jsonc'
import * as mdx from 'eslint-plugin-mdx'
import jsonParser from 'jsonc-eslint-parser'

export default [
  ...configs.recommended,
  {
    files: ['src/**/*.{json,json5,jsonc}'],
    languageOptions: { parser: jsonParser },
    plugins: { jsonc },
    rules: {
      'prettier/prettier': 'off',
      'eol-last': ['error', 'never'],
      'no-multiple-empty-lines': ['error', { max: 0, maxEOF: 0 }],
    },
  },
  {
    ...mdx.flat,
    files: ['src/**/*.{md,mdx}'],
    processor: mdx.createRemarkProcessor({
      lintCodeBlocks: true,
    }),
  },
  {
    ...mdx.flatCodeBlocks,
    files: ['src/**/*.{md,mdx}/*.{js,jsx,ts,tsx}'],
    languageOptions: { parser: tsParser },
    rules: {
      ...mdx.flatCodeBlocks.rules,
      'react/jsx-no-undef': 'off',
      semi: ['error', 'never'],
      quotes: ['error', 'single', { avoidEscape: true }],
      'react/jsx-tag-spacing': ['error', { beforeClosing: 'never' }],
    },
  },
]
