import { RuleTester } from '@typescript-eslint/rule-tester'
import { describe } from 'bun:test'

import { preferMediaShorthand } from '../index'

describe('prefer-media-shorthand rule', () => {
  const ruleTester = new RuleTester({
    languageOptions: {
      ecmaVersion: 'latest',
      parserOptions: { ecmaFeatures: { jsx: true } },
    },
  })
  const imports = 'import { Box, css } from "@devup-ui/react";\n'
  ruleTester.run('prefer-media-shorthand rule', preferMediaShorthand, {
    valid: [
      { code: `${imports}<Box _motionReduce={{ transition: 'none' }} />` },
      { code: `${imports}<Box _media={{ '(min-width: 500px)': { p: 1 } }} />` },
      { code: `${imports}<Box _media={query} />` },
      { code: `${imports}<Box _media />` },
      { code: `${imports}<Box {...props} onClick={f} />` },
      { code: `${imports}css({ '@media (hover: none)': { color: 'red' } })` },
      { code: `${imports}css({ [key]: { color: 'red' }, 1: 2, ...rest })` },
      {
        code: `${imports}css({ _media: { [key]: { color: 'red' }, ...rest } })`,
      },
      { code: `${imports}css({ '@mediaprint': { color: 'red' } })` },
      { code: `${imports}css(styles)` },
      {
        code: `import { Box } from "other";\n<Box _media={{ print: { p: 1 } }} />`,
      },
      { code: `css({ '@media print': { color: 'red' } })` },
    ],
    invalid: [
      {
        code: `${imports}<Box _media={{ '(prefers-reduced-motion: reduce)': { transition: 'none' } }} />`,
        output: `${imports}<Box _motionReduce={{ transition: 'none' }} />`,
        errors: [
          {
            messageId: 'preferMediaShorthand',
            data: { shorthand: '_motionReduce' },
          },
        ],
      },
      {
        code: `${imports}css({ _media: { print: { color: 'red' } } })`,
        output: `${imports}css({ _print: { color: 'red' } })`,
        errors: [{ messageId: 'preferMediaShorthand' }],
      },
      {
        code: `${imports}css({ _media: { '(orientation:portrait)': { p: 1 }, '(min-width: 1px)': { p: 2 } } })`,
        output: null,
        errors: [
          {
            messageId: 'preferMediaShorthand',
            data: { shorthand: '_portrait' },
          },
        ],
      },
      {
        code: `${imports}css({ '@media (forced-colors: active)': { color: 'red' } })`,
        output: `${imports}css({ _forcedColors: { color: 'red' } })`,
        errors: [{ messageId: 'preferMediaShorthand' }],
      },
      {
        code: `${imports}css({ ['@media screen']: { color: 'red' } })`,
        output: null,
        errors: [{ messageId: 'preferMediaShorthand' }],
      },
      {
        code: `${imports}<Box _hover={{ '@media Print': { color: 'red' } }} />`,
        output: `${imports}<Box _hover={{ _print: { color: 'red' } }} />`,
        errors: [{ messageId: 'preferMediaShorthand' }],
      },
    ],
  })
})
