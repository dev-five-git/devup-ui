import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterAll, afterEach, beforeEach, describe, expect, it } from 'bun:test'

import {
  createWasm,
  type DevupWasm,
  setWasmForTesting,
  withModuleResolver,
} from '../wasm'

const originalCwd = process.cwd()
const roots: string[] = []

function project(): string {
  const root = mkdtempSync(join(tmpdir(), 'devup-next-wasm-'))
  roots.push(root)
  writeFileSync(join(root, 'package.json'), '{}')
  return root
}

function extract(engine: DevupWasm, filename: string, source: string) {
  return engine.codeExtract(
    filename,
    source,
    '@devup-ui/react',
    'df',
    false,
    false,
    false,
    {},
  )
}

beforeEach(() => setWasmForTesting(undefined))
afterEach(() => {
  process.chdir(originalCwd)
  setWasmForTesting(undefined)
})
afterAll(() => {
  for (const root of roots) rmSync(root, { recursive: true, force: true })
})

describe('real per-app WASM namespaces', () => {
  it('keeps prefix and theme changes private when another app extracts', () => {
    const a = createWasm()
    const b = createWasm()

    a.setPrefix('app-a-')
    a.registerTheme({ colors: { default: { primary: 'red' } } })
    using outputA = extract(
      a,
      'page.tsx',
      'import { Box } from "@devup-ui/react"; const x = <Box bg="$primary" />',
    )
    using outputB = extract(
      b,
      'page.tsx',
      'import { Box } from "@devup-ui/react"; const x = <Box bg="blue" />',
    )

    expect(a).not.toBe(b)
    expect(a.Output).not.toBe(b.Output)
    expect(outputA).toBeInstanceOf(a.Output)
    expect(outputA).not.toBeInstanceOf(b.Output)
    expect(outputA.code).toContain('app-a-')
    expect(outputB.code).not.toContain('app-a-')
    expect(a.getCss(undefined, false)).toContain('--primary:red')
    expect(b.getCss(undefined, false)).not.toContain('--primary')
  })

  it('keeps canonical and number maps private when another app allocates names', () => {
    const a = createWasm()
    const b = createWasm()
    const source =
      'import { Box } from "@devup-ui/react"; const x = <Box bg="red" />'

    a.importCanonicalMap({ 'page.tsx': 'canonical.tsx' })
    a.importFileMap({ 'canonical.tsx': 42 })
    a.importClassMap({ reserved: { atom: 99 } })
    using outputA = extract(a, 'page.tsx', source)
    using outputB = extract(b, 'page.tsx', source)

    expect(JSON.parse(a.exportCanonicalMap())).toEqual({
      'page.tsx': 'canonical.tsx',
    })
    expect(JSON.parse(b.exportCanonicalMap())).toEqual({})
    expect(JSON.parse(a.exportFileMap())).toMatchObject({ 'canonical.tsx': 42 })
    expect(JSON.parse(a.exportClassMap())).toHaveProperty('reserved.atom', 99)
    expect(JSON.parse(b.exportFileMap())).toEqual({ 'page.tsx': 0 })
    expect(JSON.parse(b.exportClassMap())).not.toHaveProperty('reserved')
    expect(outputA.code).not.toBe(outputB.code)
    expect(b.getCss(0, false)).toContain('background:red')
  })

  it('keeps route and hoist state private when apps use the same filenames', () => {
    const a = createWasm()
    const b = createWasm()
    const source =
      'import { Box } from "@devup-ui/react"; const x = <Box bg="red" />'
    a.setAtomHoist(2)
    a.importFileRoutes({ 'a.tsx': [0], 'b.tsx': [1] })

    extract(a, 'a.tsx', source).free()
    extract(a, 'b.tsx', source).free()
    extract(b, 'a.tsx', source).free()
    extract(b, 'b.tsx', source).free()

    expect(a.getCss(undefined, false)).toContain('background:red')
    expect(a.getCss(0, false)).not.toContain('background:red')
    expect(b.getCss(undefined, false)).not.toContain('background:red')
    expect(b.getCss(0, false)).toContain('background:red')
    expect(b.getCss(1, false)).toContain('background:red')
  })

  it('captures explicit resolver roots and POSIX ids before cwd changes', () => {
    const rootA = project()
    const rootB = project()
    mkdirSync(join(rootA, 'src'))
    mkdirSync(join(rootB, 'src'))
    writeFileSync(join(rootA, 'src/color.ts'), 'export const color = "red"')
    writeFileSync(join(rootB, 'src/color.ts'), 'export const color = "blue"')
    const a = withModuleResolver(createWasm(), rootA)
    const b = withModuleResolver(createWasm(), rootB)
    process.chdir(rootB)
    const source =
      'import { Box } from "@devup-ui/react"; import { color } from "./color"; const x = <Box bg={color} />'

    using outputA = extract(a, 'src/page.tsx', source)
    using outputB = extract(b, 'src/page.tsx', source)

    expect(outputA.dependencies).toEqual(['src/color.ts'])
    expect(outputB.dependencies).toEqual(['src/color.ts'])
    expect(a.getCss(0, false)).toContain('background:red')
    expect(b.getCss(0, false)).toContain('background:blue')
  })

  it('keeps resolver replacement private when another app imports a constant', () => {
    const a = createWasm()
    const b = createWasm()
    const source =
      'import { Box } from "@devup-ui/react"; import { color } from "./color"; const x = <Box bg={color} />'
    a.setModuleResolver(() => ({
      path: 'color.ts',
      code: 'export const color = "red"',
    }))
    b.setModuleResolver(() => ({
      path: 'color.ts',
      code: 'export const color = "blue"',
    }))

    a.setModuleResolver(() => ({
      path: 'color.ts',
      code: 'export const color = "green"',
    }))
    extract(a, 'page.tsx', source).free()
    extract(b, 'page.tsx', source).free()

    expect(a.getCss(0, false)).toContain('background:green')
    expect(b.getCss(0, false)).toContain('background:blue')
  })

  it('returns same-realm errors when real WASM rejects invalid input', () => {
    const engine = createWasm()
    expect.assertions(1)

    try {
      engine.importFileMap({ broken: 'not-a-number' })
    } catch (error) {
      if (!(error instanceof Error)) throw error
      expect(Object.getPrototypeOf(error)).toBe(Error.prototype)
    }
  })

  it('uses physical-install entry, local require and dirname with an explicit root', () => {
    const root = project()
    const modules = join(
      root,
      'node_modules/.bun/next-plugin/node_modules/@devup-ui',
    )
    const plugin = join(modules, 'next-plugin')
    const engineDir = join(modules, 'wasm')
    const scope = join(root, 'node_modules/@devup-ui')
    mkdirSync(plugin, { recursive: true })
    mkdirSync(engineDir, { recursive: true })
    mkdirSync(scope, { recursive: true })
    writeFileSync(join(plugin, 'package.json'), '{}')
    writeFileSync(join(engineDir, 'package.json'), '{"main":"engine.cjs"}')
    const realEntry = createRequire(import.meta.path).resolve('@devup-ui/wasm')
    copyFileSync(
      join(dirname(realEntry), 'index_bg.wasm'),
      join(engineDir, 'index_bg.wasm'),
    )
    writeFileSync(join(engineDir, 'prefix.cjs'), 'module.exports = "physical-"')
    writeFileSync(
      join(engineDir, 'engine.cjs'),
      readFileSync(realEntry, 'utf8') +
        '\nexports.setPrefix(require("./prefix.cjs") + require("path").basename(__filename));',
    )
    writeFileSync(join(root, 'color.ts'), 'export const color = "purple"')
    symlinkSync(
      plugin,
      join(scope, 'next-plugin'),
      process.platform === 'win32' ? 'junction' : 'dir',
    )

    const a = createWasm(root)
    const b = createWasm(root)
    a.setPrefix('changed')

    expect(b.getPrefix()).toBe('physical-engine.cjs')
    expect(a.getPrefix()).toBe('changed')
    using output = extract(
      b,
      'page.tsx',
      'import { Box } from "@devup-ui/react"; import { color } from "./color"; const x = <Box bg={color} />',
    )
    expect(output.code).toContain('physical-engine.cjs')
    expect(output.dependencies).toEqual(['color.ts'])
    expect(b.getCss(0, false)).toContain('background:purple')
  })
})
