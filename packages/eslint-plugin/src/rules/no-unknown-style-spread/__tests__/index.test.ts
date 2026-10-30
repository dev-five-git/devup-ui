import { join } from 'node:path'

import { describe } from 'bun:test'

import { noUnknownStyleSpread } from '../index'
import { potentiallyKnownCases, runtimeCases } from './runtime-cases'
import { TypedRuleTester } from './typed-rule-tester'

describe('no-unknown-style-spread', () => {
  const root = join(import.meta.dir, 'fixtures')
  const filename = join(root, 'case.tsx')
  const tester = new TypedRuleTester({
    languageOptions: {
      parserOptions: { projectService: true, tsconfigRootDir: root },
    },
  })
  const imports =
    'import { Box, Flex, type DevupProps } from "@devup-ui/react"; import type * as React from "react";\n'
  const invalidTypes = [
    'Pick<DevupProps, "bg">',
    '{ p?: number }',
    '{ color?: string }',
    '{ width?: string }',
    'React.HTMLAttributes<HTMLDivElement> | { p?: number }',
    '{ bg?: string } | React.HTMLAttributes<HTMLDivElement>',
    'React.HTMLAttributes<HTMLDivElement> & { bg?: string }',
    'React.HTMLAttributes<HTMLDivElement> & DevupProps',
    'Record<string, unknown>',
    'Record<string, number>',
    '{ _hover?: DevupProps }',
    '{ selectors?: Record<string, DevupProps> }',
    '{ [key: `@media ${string}`]: DevupProps }',
    '{ [key: `_theme${string}`]: DevupProps }',
    'any',
    'unknown',
    'object',
    '{}',
  ]
  const domTypes = [
    'React.HTMLAttributes<HTMLDivElement>',
    'React.ImgHTMLAttributes<HTMLImageElement>',
    'React.SVGProps<SVGSVGElement>',
    'React.ComponentProps<"div">',
    'React.ComponentProps<"img">',
    'React.ComponentProps<"svg">',
    'Pick<React.HTMLAttributes<HTMLDivElement>, "color">',
    'Pick<React.ImgHTMLAttributes<HTMLImageElement>, "width">',
    'Partial<React.ImgHTMLAttributes<HTMLImageElement>>',
    'Readonly<React.HTMLAttributes<HTMLDivElement>>',
    'Required<Pick<React.HTMLAttributes<HTMLDivElement>, "color">>',
    'Omit<React.HTMLAttributes<HTMLDivElement>, "id">',
    'React.HTMLAttributes<HTMLDivElement> & { trackingId?: string }',
    'React.HTMLAttributes<HTMLDivElement> | React.ImgHTMLAttributes<HTMLImageElement>',
    '{ className?: string; style?: React.CSSProperties }',
    '{ id?: string; onClick?: () => void; "data-x"?: string; "aria-label"?: string }',
    '{ [key: `data-${string}`]: string }',
    '{ [key: `aria-${string}`]: string }',
    'Record<number, string>',
    '{ [key: symbol]: unknown }',
    'React.HTMLAttributes<HTMLDivElement> & object',
    'React.HTMLAttributes<HTMLDivElement> & {}',
    '{ bg?: undefined }',
    '{ p: never }',
    'never',
  ]

  // Given real Devup/React types; When linting each source; Then only proven runtime style spreads report.
  tester.run('no-unknown-style-spread', noUnknownStyleSpread, {
    valid: [
      ...potentiallyKnownCases,
      ...domTypes.map(
        (type) => `function f(props: ${type}) { return <Box {...props} /> }`,
      ),
      'type Named = React.HTMLAttributes<HTMLDivElement>; function f(props: Named) { return <Box {...props} /> }',
      'interface Named extends React.HTMLAttributes<HTMLDivElement> {}; function f(props: Named) { return <Box {...props} /> }',
      'function f<T extends React.HTMLAttributes<HTMLDivElement>>(props: T) { return <Box {...props} /> }',
      'function f<T extends React.HTMLAttributes<HTMLDivElement>>(props: React.HTMLAttributes<HTMLDivElement> & T) { return <Box {...props} /> }',
      'function f<T extends { trackingId?: string }>(props: React.HTMLAttributes<HTMLDivElement> & T) { return <Box {...props} /> }',
      'function f(props: DevupProps) { return <div {...props} /> }',
      'function f(Box: React.ElementType, props: DevupProps) { return <Box {...props} /> }',
      'import * as UI from "@devup-ui/react"; function f(UI: { Box: React.ElementType }, props: DevupProps) { return <UI.Box {...props} /> }',
      'import { Box as Other } from "other-package"; function f(props: DevupProps) { return <Other {...props} /> }',
      'import { ThemeScript } from "@devup-ui/react"; function f(props: DevupProps) { return <ThemeScript {...props} /> }',
      'import type { Box as Typed } from "@devup-ui/react"; function f(props: DevupProps) { return <Typed {...props} /> }',
      'import type * as UI from "@devup-ui/react"; function f(props: DevupProps) { return <UI.Box {...props} /> }',
      'function f() { return <Box {...arguments} /> }',
      'function f() { return <Box {...unresolvedBinding} /> }',
      'function f(props: React.HTMLAttributes<HTMLDivElement>) { return <Box className="before" {...props} style={{ color: "red" }} /> }',
      'function f(props: React.HTMLAttributes<HTMLDivElement>) { return <Box style={{ color: "red" }} {...props} className="after" /> }',
    ].map((code) => ({ code: imports + code, filename })),
    invalid: [
      ...runtimeCases,
      ...invalidTypes.map(
        (type) => `function f(props: ${type}) { return <Box {...props} /> }`,
      ),
      'interface Named { bg?: string }; function f(props: Named) { return <Box {...props} /> }',
      'interface Named extends React.HTMLAttributes<HTMLDivElement> { color?: string }; function f(props: Named) { return <Box {...props} /> }',
      'function f<T extends DevupProps>(props: T) { return <Box {...props} /> }',
      'function f<T extends Pick<DevupProps, "bg">>(props: T) { return <Box {...props} /> }',
      'function f<T>(props: T) { return <Box {...props} /> }',
      'function f<T extends object>(props: T) { return <Box {...props} /> }',
      'function f<T>(props: React.HTMLAttributes<HTMLDivElement> & T) { return <Box {...props} /> }',
      'function f<T extends object>(props: React.HTMLAttributes<HTMLDivElement> & T) { return <Box {...props} /> }',
      'function f<T extends {}>(props: React.HTMLAttributes<HTMLDivElement> & T) { return <Box {...props} /> }',
      'function f<T extends unknown>(props: React.HTMLAttributes<HTMLDivElement> & T) { return <Box {...props} /> }',
      'function f<T extends any>(props: React.HTMLAttributes<HTMLDivElement> & T) { return <Box {...props} /> }',
      'import { Box as B } from "@devup-ui/react"; function f(props: DevupProps) { return <B {...props} /> }',
      'import UI from "@devup-ui/react"; function f(props: DevupProps) { return <UI.Box {...props} /> }',
      'function f(props: DevupProps) { return <Flex {...props} /> }',
      ...[
        'Box',
        'Button',
        'Center',
        'Flex',
        'Grid',
        'Image',
        'Input',
        'Text',
        'VStack',
      ].map(
        (component) =>
          `import * as UI from "@devup-ui/react"; function f(props: DevupProps) { return <UI.${component} {...props} /> }`,
      ),
      'declare module "react" { interface HTMLAttributes<T> { bg?: string } }; function f(props: React.HTMLAttributes<HTMLDivElement>) { return <Box {...props} /> }',
      'declare module "@devup-ui/react" { interface DevupCustomShorthands { insetX?: number } }; function f(props: { insetX?: number }) { return <Box {...props} /> }',
    ]
      .map((code) => ({
        code: imports + code,
        filename,
        errors: [
          {
            messageId: 'unknownStyleSpread' as const,
            type: 'JSXSpreadAttribute' as const,
          },
        ],
        output: null,
      }))
      .concat([
        {
          code:
            imports +
            'function f(props: DevupProps) { return <><Box {...props} /><Box {...props} /></> }',
          filename,
          errors: [
            {
              messageId: 'unknownStyleSpread' as const,
              type: 'JSXSpreadAttribute' as const,
            },
            {
              messageId: 'unknownStyleSpread' as const,
              type: 'JSXSpreadAttribute' as const,
            },
          ],
          output: null,
        },
      ]),
  })
})
