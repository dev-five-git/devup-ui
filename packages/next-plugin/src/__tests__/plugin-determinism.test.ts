import { describe, expect, it } from 'bun:test'

import type { CoordinatorOptions } from '../coordinator-options'
import { DevupUI } from '../plugin'
import { createWasm, type DevupWasm } from '../wasm'
import { installProjectHooks, makeProject } from './project'
import { installTurboHarness } from './turbo-harness'

installProjectHooks()
const harness = installTurboHarness()

const page = (body: string) =>
  `import { Box, keyframes } from '@devup-ui/react'\n${body}\n`

const routes: Record<string, string> = {
  'src/app/a/page.tsx': page(
    'const spin = keyframes({ from: { opacity: 0 }, to: { opacity: 1 } })\nexport default () => <Box bg="red" p={4} animation={`${spin} 1s`} _hover={{ bg: "blue" }} />',
  ),
  'src/app/b/page.tsx': page(
    'const move = keyframes({ from: { top: 0 }, to: { top: 10 } })\nexport default () => <Box bg="red" m={2} animation={`${move} 2s`} />',
  ),
  'src/app/c/page.tsx': page(
    'export default () => <Box p={4} color="green" />',
  ),
}

const scenarios = [
  { name: 'per-file css', options: {} },
  { name: 'single css', options: { singleCss: true } },
  { name: 'atom hoisting', options: { atomHoist: 2 } },
  {
    name: 'atom hoisting with a single css',
    options: { atomHoist: 2, singleCss: true },
  },
]

function reversed(files: Record<string, string>): Record<string, string> {
  return Object.fromEntries(Object.entries(files).reverse())
}

function request(start: CoordinatorOptions, order: string[]) {
  const codes: Record<string, string> = {}
  for (const filename of order) {
    const { source } = start.prewarmedOutputs!.get(filename)!
    const cssDir = './df/devup-ui'
    using output = start.wasm.codeExtract(
      filename,
      source,
      start.package,
      cssDir,
      start.singleCss,
      false,
      true,
      start.importAliases,
    )
    codes[filename] = output.code
  }
  return codes
}

function state(engine: DevupWasm) {
  const files = JSON.parse(engine.exportFileMap()) as Record<string, number>
  return {
    classMap: engine.exportClassMap(),
    fileMap: engine.exportFileMap(),
    base: engine.getCss(undefined, false),
    buckets: Object.values(files)
      .sort((a, b) => a - b)
      .map((number) => engine.getCss(number, false)),
  }
}

function fresh(order: string[]) {
  const engine = createWasm(makeProject())
  for (const filename of order) {
    using output = engine.codeExtract(
      filename,
      routes[filename]!,
      '@devup-ui/react',
      './df/devup-ui',
      false,
      false,
      true,
      {},
    )
    expect(output.code).not.toBe('')
  }
  return engine.exportClassMap()
}

describe('prewarm makes names independent of the order requests arrive in', () => {
  const names = Object.keys(routes)

  it('proves the order matters when nothing is prewarmed', () => {
    expect(fresh(names)).not.toBe(fresh([...names].reverse()))
  })

  describe.each(['development', 'production'] as const)('%s', (phase) => {
    it.each(scenarios)(
      'gives the same classes, files and sheets for any request order: $name',
      ({ options }) => {
        process.env.NODE_ENV = phase
        process.chdir(makeProject(routes))
        DevupUI({}, options)
        process.chdir(makeProject(reversed(routes)))
        DevupUI({}, options)
        const [forward, backward] = harness.starts

        const prewarmed = state(forward!.wasm)
        const forwardCodes = request(forward!, names)
        const backwardCodes = request(backward!, [...names].reverse())

        expect(state(backward!.wasm)).toEqual(prewarmed)
        expect(state(forward!.wasm)).toEqual(prewarmed)
        expect(backwardCodes).toEqual(forwardCodes)
        for (const filename of names) {
          expect(backward!.prewarmedOutputs!.get(filename)!.code).toBe(
            forward!.prewarmedOutputs!.get(filename)!.code,
          )
        }
      },
    )
  })

  it('numbers the files by path and the later ones after them in development', () => {
    process.env.NODE_ENV = 'development'
    const root = makeProject(routes)
    process.chdir(root)
    DevupUI({}, {})
    const [start] = harness.starts

    expect(JSON.parse(start!.wasm.exportFileMap())).toEqual({
      'src/app/a/page.tsx': 0,
      'src/app/b/page.tsx': 1,
      'src/app/c/page.tsx': 2,
    })
    using later = start!.wasm.codeExtract(
      'src/app/aaa.tsx',
      page('export const X = () => <Box w={1} />'),
      '@devup-ui/react',
      './df/devup-ui',
      false,
      false,
      true,
      {},
    )
    expect(later.code).not.toBe('')
    expect(JSON.parse(start!.wasm.exportFileMap())).toMatchObject({
      'src/app/aaa.tsx': 3,
    })
  })

  it('numbers root-level app and pages files in path order too', () => {
    process.chdir(
      makeProject({
        'pages/z.tsx': routes['src/app/c/page.tsx']!,
        'app/page.tsx': routes['src/app/b/page.tsx']!,
        'src/app/a/page.tsx': routes['src/app/a/page.tsx']!,
      }),
    )

    DevupUI({}, {})

    expect(JSON.parse(harness.starts[0]!.wasm.exportFileMap())).toEqual({
      'app/page.tsx': 0,
      'pages/z.tsx': 1,
      'src/app/a/page.tsx': 2,
    })
  })
})
