import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, relative } from 'node:path'

import {
  buildStaticImportGraph,
  createModuleResolver,
} from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { describe, expect, it } from 'bun:test'

import { flushCoordinatorWrites, resetCoordinator } from '../coordinator'
import { DevupUI, resetTurboSetupCacheForTesting } from '../plugin'
import { collectProductionPrewarmFiles } from '../prewarm'
import {
  resetWasmForTesting,
  setWasmForTesting,
  withModuleResolver,
} from '../wasm'

describe.each(['next', 'webpack'])(
  '%s unreadable Markdown parity',
  (adapter) => {
    it.each([
      './Description.mdx',
      './Description',
      '@description',
      'plain-doc',
    ])(
      'prewarms JS importing %s without reading Markdown into WASM',
      (request) => {
        // Given the plain Markdown and JS importer exercised against main.
        const root = mkdtempSync(join(tmpdir(), 'devup-ui-markdown-parity-'))
        const cwd = process.cwd()
        try {
          mkdirSync(join(root, 'node_modules/plain-doc'), { recursive: true })
          const markdown =
            '# Plain description\n\nThis is **Markdown**, not JavaScript.\n'
          writeFileSync(join(root, 'Description.mdx'), markdown)
          writeFileSync(
            join(root, 'node_modules/plain-doc/Description.mdx'),
            markdown,
          )
          writeFileSync(
            join(root, 'node_modules/plain-doc/package.json'),
            JSON.stringify({ name: 'plain-doc', exports: './Description.mdx' }),
          )
          writeFileSync(
            join(root, 'tsconfig.json'),
            JSON.stringify({
              compilerOptions: {
                paths: { '@description': ['./Description.mdx'] },
              },
            }),
          )
          const source = `import { Box, css } from '@devup-ui/react';\nimport Description from '${request}';\nexport const card = css({ color: 'red' });\nexport default function Page() { return <Box p={4} className={card} styleOrder={1}><Description /></Box> }`
          writeFileSync(join(root, 'page.tsx'), source)
          process.chdir(root)
          wasm.resetStateForTesting()
          const resolver = createModuleResolver({
            ...(adapter === 'webpack' ? { includeMdx: ['.mdx'] } : {}),
            toId: (path) => relative(root, path).replaceAll('\\', '/'),
          })
          if (adapter === 'next') withModuleResolver(wasm)
          else wasm.setModuleResolver(resolver)

          // When Next selects production prewarm inputs and extracts the JS.
          const files = collectProductionPrewarmFiles({
            cwd: root,
            graph: buildStaticImportGraph(root, undefined, { cwd: root }),
            expectedBaseFiles: ['page.tsx'],
            libPackage: '@devup-ui/react',
            include: [],
          })
          using output = wasm.codeExtract(
            'page.tsx',
            source,
            '@devup-ui/react',
            'devup-ui.css',
            false,
            true,
            false,
            {},
          )

          // Then main's unreadable result and exact code/CSS bytes are retained.
          expect(files).toEqual(['page.tsx'])
          expect(resolver(request, 'page.tsx')).toBeUndefined()
          expect(output.code).toBe(
            `import "devup-ui.css/devup-ui.css";\nimport "devup-ui.css/devup-ui-0.css";\nimport Description from "${request}";\nexport const card = "a-a";\nexport default function Page() {\n\treturn <div className={\`\${card || ""} a-b\`}><Description /></div>;\n}\n`,
          )
          expect(output.css).toBe(
            '/*! devup-ui v1.0.83, | Apache License 2.0 | https://devup-ui.com */@layer o1{.a-b{padding:16px}}.a-a{color:red}',
          )
        } finally {
          wasm.resetStateForTesting()
          process.chdir(cwd)
          rmSync(root, { recursive: true, force: true })
        }
      },
    )
  },
)

it('resolves the JS companion rather than reading an existing Markdown file', () => {
  // Given both plain Markdown and the JavaScript completion main can read.
  const root = mkdtempSync(join(tmpdir(), 'devup-ui-markdown-companion-'))
  try {
    writeFileSync(join(root, 'Description.mdx'), '# Not JavaScript')
    const path = join(root, 'Description.mdx.ts')
    writeFileSync(path, 'export const color = "purple"')
    // When resolving the explicit Markdown request without a preparer.
    const resolved = createModuleResolver({ cwd: root })(
      './Description.mdx',
      'page.tsx',
    )
    // Then the real JS source wins, not a blank or ignored substitute.
    expect(resolved).toEqual({ path, code: 'export const color = "purple"' })
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

it('finishes actual production Turbo setup when a prewarmed JS page imports plain MDX', async () => {
  // Given a production Next source tree, real engine and real Markdown.
  const root = mkdtempSync(join(tmpdir(), 'devup-ui-markdown-prewarm-'))
  const cwd = process.cwd()
  const env = { ...process.env }
  try {
    mkdirSync(join(root, 'src/app'), { recursive: true })
    writeFileSync(join(root, 'package.json'), '{"private":true}')
    writeFileSync(
      join(root, 'src/app/Description.mdx'),
      '# Plain description\n',
    )
    writeFileSync(
      join(root, 'src/app/page.tsx'),
      `import { Box } from '@devup-ui/react'; import Description from './Description.mdx'; export default function Page() { return <Box p={4}><Description /></Box> }`,
    )
    process.chdir(root)
    Object.assign(process.env, { TURBOPACK: '1', NODE_ENV: 'production' })
    wasm.resetStateForTesting()
    setWasmForTesting(withModuleResolver(wasm))

    // When the actual config wrapper runs its synchronous prewarm loop.
    const config = DevupUI({}, { singleCss: true })

    // Then setup finishes with the page's CSS, without compiling its Markdown.
    expect(config.turbopack?.rules).toBeDefined()
    expect(wasm.getCss(null, false)).toContain('padding:16px')
  } finally {
    await flushCoordinatorWrites()
    resetCoordinator()
    resetTurboSetupCacheForTesting()
    resetWasmForTesting()
    wasm.resetStateForTesting()
    process.chdir(cwd)
    process.env = env
    rmSync(root, { recursive: true, force: true })
  }
})
