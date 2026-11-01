import { join } from 'node:path'

import { describe } from 'bun:test'

import { noUnknownStyleSpread } from '../index'
import { TypedRuleTester } from './typed-rule-tester'

describe('runtime style capability regressions', () => {
  const root = join(import.meta.dir, 'fixtures')
  const filename = join(root, 'case.tsx')
  const tester = new TypedRuleTester({
    languageOptions: {
      parserOptions: { projectService: true, tsconfigRootDir: root },
    },
  })
  const code = (type: string) =>
    `import { Box, type DevupProps } from '@devup-ui/react'; import type * as React from 'react'; function f(props: ${type}) { return <Box {...props} /> }`

  // Given merged React declarations; When forwarding runtime props; Then only original intrinsic attributes are trusted.
  const elementAugmentation =
    'interface ReactElement<P = unknown, T extends string | React.JSXElementConstructor<any> = string | React.JSXElementConstructor<any>> { trace?: true }'
  tester.run('DOM origin survives paired augmentation', noUnknownStyleSpread, {
    valid: [
      {
        code:
          `declare module 'react' { ${elementAugmentation} };` +
          code(
            'Pick<React.ImgHTMLAttributes<HTMLImageElement>, "color" | "width">',
          ),
        filename,
      },
      {
        code:
          'declare module "react" { namespace JSX { interface IntrinsicElements { input: React.DetailedHTMLProps<React.InputHTMLAttributes<HTMLInputElement>, HTMLInputElement> } } };' +
          code('Pick<React.InputHTMLAttributes<HTMLInputElement>, "width">'),
        filename,
      },
    ],
    invalid: [
      {
        code:
          `declare module 'react' { ${elementAugmentation}; interface HTMLAttributes<T> { bg?: string } };` +
          code('React.HTMLAttributes<HTMLDivElement>'),
        filename,
        errors: [{ messageId: 'unknownStyleSpread' }],
        output: null,
      },
      {
        code:
          `declare module 'react' { ${elementAugmentation}; namespace JSX { interface IntrinsicElements { workerCustom: React.PointerEvent<HTMLElement> } } };` +
          code('Pick<React.PointerEvent<HTMLElement>, "width">'),
        filename,
        errors: [{ messageId: 'unknownStyleSpread' }],
        output: null,
      },
    ],
  })

  // Given actual React/Devup types; When linting runtime inputs; Then effective DOM/key domains determine diagnostics.
  tester.run('no-unknown-style-spread', noUnknownStyleSpread, {
    valid: [
      '{ bg?: string } & { bg?: undefined }',
      'React.HTMLAttributes<HTMLDivElement> & { bg?: string } & { bg?: undefined }',
      '{ [key: `@mediaX${string}`]: DevupProps }',
      '{ [key: `@screen ${string}`]: DevupProps }',
      '{ [key: `data-${string} (min-width: 1px)`]: DevupProps }',
      '{ [key: `aria-${string}`]: DevupProps }',
      '{ [key: `@${number} (min-width: 1px)`]: DevupProps }',
      'Record<number, DevupProps>',
      '{ [key: symbol]: DevupProps }',
      'Record<string, undefined>',
      'Record<string, null>',
      'Record<string, never>',
      '{ [key: `@media ${string}`]: undefined }',
      '{ bg?: string } & Record<string, undefined>',
      '{ [key: `@media ${number}`]: unknown } & { [key: `@media ${string}`]: undefined }',
      '{ [key: `data-${number}`]: unknown } & { [key: `@media ${number}`]: undefined }',
    ].map((type) => ({ code: code(type), filename })),
    invalid: [
      'Pick<React.PointerEvent<HTMLElement>, "width">',
      '{ [key: `@${string} (min-width: 1px)`]: DevupProps }',
      '{ [key: `${string}media (min-width: 1px)`]: DevupProps }',
      '{ [key: `@${string} (min-width: ${string})`]: DevupProps }',
      '{ [key: `@media ${string}`]: unknown } & { [key: `@media ${number}`]: undefined }',
      '{ [key: `@supports ${string}`]: unknown } & { [key: `@supports ${number}`]: undefined }',
      '{ [key: `@container ${string}`]: unknown } & { [key: `@container ${number}`]: undefined }',
      '{ [key: `@media ${number}`]: unknown } & { "@media 0"?: undefined }',
      '{ bg?: string } & { [key: `${number}`]: undefined }',
    ]
      .map((type) => ({
        code: code(type),
        filename,
        errors: [{ messageId: 'unknownStyleSpread' as const }],
        output: null,
      }))
      .concat([
        {
          code:
            'declare module "react" { namespace JSX { interface IntrinsicElements { workerCustom: React.PointerEvent<HTMLElement> } } };' +
            code('Pick<React.PointerEvent<HTMLElement>, "width">'),
          filename,
          errors: [{ messageId: 'unknownStyleSpread' as const }],
          output: null,
        },
      ]),
  })

  // Given finite generic constraints; When intersected; Then only surviving style capabilities report.
  const genericCode = (constraint: string, type: string) =>
    `import { Box, type DevupProps } from '@devup-ui/react'; import type * as React from 'react'; function render<T extends ${constraint}>(rest: ${type}) { return <Box {...rest} /> }`
  tester.run('finite generic intersections', noUnknownStyleSpread, {
    valid: [
      'T & { bg?: undefined }',
      'React.HTMLAttributes<HTMLDivElement> & T & { bg?: undefined }',
    ].map((type) => ({
      code: genericCode('Pick<DevupProps, "bg">', type),
      filename,
    })),
    invalid: (
      [
        ['Pick<DevupProps, "bg">', 'T & { id?: string }'],
        [
          'Pick<DevupProps, "bg" | "p">',
          'React.HTMLAttributes<HTMLDivElement> & T & { bg?: undefined }',
        ],
        ['Pick<DevupProps, "bg">', 'T & { bg?: undefined; p?: number }'],
        ['Record<string, unknown>', 'T & { bg?: undefined }'],
      ] as const
    ).map(([constraint, type]) => ({
      code: genericCode(constraint, type),
      filename,
      errors: [{ messageId: 'unknownStyleSpread' as const }],
      output: null,
    })),
  })

  // Given constrained union/index capabilities; When intersected; Then effective absence masks only eliminated capabilities.
  const dom = 'React.HTMLAttributes<HTMLDivElement>'
  const union = `(${dom} | { bg?: string })`
  const media = '{ [key: `@media ${string}`]: unknown }'
  const query = '{ [key: `@${string} (min-width: 1px)`]: unknown }'
  const maskedQueries =
    '{ [key: `@media ${string}(min-width: 1px)` | `@supports ${string}(min-width: 1px)` | `@container ${string}(min-width: 1px)`]: undefined }'
  tester.run('generic capability composition', noUnknownStyleSpread, {
    valid: [
      ['Record<string, unknown>', 'T & Record<string, undefined>'],
      ['Record<string, unknown>', 'T & Record<string, null>'],
      ['Record<string, unknown>', 'T & Record<string, never>'],
      ['Record<string, unknown>', `${dom} & T & Record<string, undefined>`],
      [media, 'T & { [key: `@media ${string}`]: undefined }'],
      [media, `${dom} & T & { [key: \`@media \${string}\`]: undefined }`],
      [media, 'T & Record<string, undefined>'],
      ['{ bg?: string }', 'T & Record<string, undefined>'],
      [union, `${dom} & T & { bg?: undefined }`],
      [union, 'T & { id?: string; bg?: undefined }'],
      [union, 'T & { bg?: null }'],
      [union, 'T & { bg?: never }'],
      [union, 'T & Record<string, undefined>'],
      [`(${dom} | { trackingId?: string })`, `${dom} & T`],
      [`(${dom} | { bg?: undefined })`, `${dom} & T`],
      [
        `(${dom} | { [key: \`@media \${string}\`]: unknown })`,
        'T & Record<string, undefined>',
      ],
      [query, `T & ${maskedQueries}`],
      [query, `T & ${maskedQueries} & ${dom}`],
    ].map(([constraint, type]) => ({
      code: genericCode(constraint, type),
      filename,
    })),
    invalid: [
      [union, `${dom} & T`],
      [union, 'T & { id?: string }'],
      [union, 'T'],
      [`({ bg?: string } | ${dom})`, `${dom} & T`],
      [
        `(${dom} | { bg?: string; p?: number })`,
        `${dom} & T & { bg?: undefined }`,
      ],
      [`(${dom} | { bg?: string } | { p?: number })`, 'T & { bg?: undefined }'],
      [union, 'T & { bg?: undefined; p?: number }'],
      ['Record<string, unknown>', 'T & Record<string, string>'],
      [media, 'T & { [key: `@media ${string}`]: DevupProps }'],
      [
        'Record<string, unknown>',
        'T & { [key: `@media ${string}`]: undefined }',
      ],
      [media, 'T & { bg?: undefined }'],
      [media, 'T & { [key: `@media print${string}`]: undefined }'],
      [`(${dom} | { [key: \`@media \${string}\`]: unknown })`, `${dom} & T`],
      [query, 'T & { [key: `@media ${string}(min-width: 1px)`]: undefined }'],
    ].map(([constraint, type]) => ({
      code: genericCode(constraint, type),
      filename,
      errors: [{ messageId: 'unknownStyleSpread' as const }],
      output: null,
    })),
  })
})
