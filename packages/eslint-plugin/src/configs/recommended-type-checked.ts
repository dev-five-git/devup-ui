import { parser } from 'typescript-eslint'

import { noUnknownStyleSpread } from '../rules/no-unknown-style-spread'
import recommended from './recommended'

const typedPlugin = {
  rules: { 'no-unknown-style-spread': noUnknownStyleSpread },
}

export const recommendedTypeChecked = [
  ...recommended,
  {
    name: '@devup-ui/recommended-type-checked',
    files: ['**/*.{ts,tsx,mts,cts}'],
    languageOptions: { parser },
    plugins: { '@devup-ui/type-checked': typedPlugin },
    rules: {
      '@devup-ui/type-checked/no-unknown-style-spread': 'error' as const,
    },
  },
]
