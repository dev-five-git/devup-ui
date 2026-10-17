import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

export const mdxLanding = createRequire(
  resolve(import.meta.dir, '../../../../apps/landing/package.json'),
)

export const guardCases = [
  ['used Box', 'import {Box} from "@devup-ui/react"\n\n<Box />', true],
  ['unused Box', 'import {Box} from "@devup-ui/react"\n\n# Plain', false],
  [
    'runtime helper',
    'import {getTheme} from "@devup-ui/react"\n\nexport const theme = getTheme\n\n# Plain',
    false,
  ],
  [
    'namespace css',
    'import * as DU from "@devup-ui/react"\n\nexport const style = DU.css({color:"red"})\n\n# Plain',
    true,
  ],
  [
    'namespace destructure',
    'import * as DU from "@devup-ui/react"\n\nexport const {css} = DU\n\n# Plain',
    true,
  ],
  [
    'opaque namespace',
    'import * as DU from "@devup-ui/react"\n\nexport const namespace = DU\n\n# Plain',
    false,
  ],
  [
    'active alias',
    'import styled from "@emotion/styled"\n\nexport const component = styled("div")\n\n# Plain',
    true,
  ],
  [
    'runtime alias',
    'import {ThemeProvider} from "styled-components"\n\n<ThemeProvider />',
    false,
  ],
  [
    'unmapped alias',
    'import {CacheProvider} from "@emotion/react"\n\n<CacheProvider />',
    false,
  ],
  [
    'ordinary alias Box',
    'import {Box} from "ordinary-provider"\n\n<Box />',
    false,
  ],
  ['actual alias Box', 'import {Box} from "opaque-provider"\n\n<Box />', true],
  [
    'shadowed namespace',
    'import * as DU from "@devup-ui/react"\n\nexport function use(DU){return DU.css()}\n\n# Plain',
    false,
  ],
] as const

export const cjsGuardCases = [
  [
    'require member',
    'export const value=require("@devup-ui/react").css({color:"red"})',
    true,
  ],
  [
    'require namespace',
    'export const DU=require("@devup-ui/react")\n\nexport const value=DU.css({color:"red"})',
    true,
  ],
  [
    'require destructure',
    'export const {css:value}=require("@devup-ui/react")',
    true,
  ],
  [
    'require runtime',
    'export const value=require("@devup-ui/react").getTheme',
    false,
  ],
  ['require opaque', 'export const value=require("@devup-ui/react")', false],
  [
    'require shadowed',
    'export function f(require){return require("@devup-ui/react").css()}',
    false,
  ],
  ['dynamic opaque', 'export const value=import("@devup-ui/react")', false],
] as const

export function mdxBuildFixture(source: string, extension = '.mdown') {
  const root = realpathSync.native(
    mkdtempSync(join(tmpdir(), 'devup-mdx-build-')),
  )
  const filename = join(root, `page${extension}`)
  writeFileSync(filename, source)
  writeFileSync(
    join(root, 'ordinary.js'),
    'export const Box = "div"; export const CacheProvider = "div"; export const ThemeProvider = "div"; export default () => "div"',
  )
  writeFileSync(join(root, 'barrel.js'), 'export * from "./forward.js"')
  writeFileSync(
    join(root, 'forward.js'),
    'export {Box as Widget} from "@devup-ui/react"',
  )
  mkdirSync(join(root, 'node_modules/skipped'), { recursive: true })
  writeFileSync(
    join(root, 'node_modules/skipped/package.json'),
    '{"name":"skipped","main":"index.js"}',
  )
  writeFileSync(
    join(root, 'node_modules/skipped/index.js'),
    'import {Box} from "@devup-ui/react"; export const component = Box',
  )
  const devup = mdxLanding.resolve('@devup-ui/react')
  const alias = {
    '@devup-ui/react': devup,
    'opaque-provider': devup,
    'ordinary-provider': join(root, 'ordinary.js'),
    '@emotion/styled': join(root, 'ordinary.js'),
    '@emotion/react': join(root, 'ordinary.js'),
    'styled-components': join(root, 'ordinary.js'),
  }
  const rule = {
    test: /\.(?:mdown|mdx)$/i,
    type: 'javascript/auto',
    use: [
      {
        loader: mdxLanding.resolve('@mdx-js/loader'),
        options: { format: 'mdx', development: false },
      },
    ],
  }
  return {
    root,
    filename,
    alias,
    rule,
    cleanup: () => rmSync(root, { recursive: true, force: true }),
  }
}
