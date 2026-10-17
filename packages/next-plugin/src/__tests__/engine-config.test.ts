import { writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'bun:test'

import { createEngineConfigurer } from '../engine-config'
import { createAppContext } from '../session'
import { createWasm } from '../wasm'
import { box, installProjectHooks, makeProject } from './project'

installProjectHooks()

const originalEnv = { ...process.env }
afterEach(() => {
  process.env = { ...originalEnv }
})

const plan = { canonicalMap: {}, fileRoutes: {}, atomThreshold: null }

function context(options = {}) {
  process.chdir(makeProject())
  return createAppContext({}, options)
}

describe('createEngineConfigurer', () => {
  it('sets every option on a fresh engine', () => {
    const app = context({
      debug: true,
      prefix: 'du-',
      shorthands: { insetX: ['left', 'right'] },
    })
    const engine = createWasm(app.root)

    createEngineConfigurer(app, {
      theme: { colors: { default: { primary: 'red' } } },
      plan: {
        canonicalMap: { 'a.tsx': 'b.tsx' },
        fileRoutes: { 'b.tsx': [0, 1] },
        atomThreshold: 2,
      },
    })(engine)

    expect(engine.isDebug()).toBe(true)
    expect(engine.getPrefix()).toBe('du-')
    expect(engine.getCss(undefined, false)).toContain('--primary:red')
    expect(JSON.parse(engine.exportCanonicalMap())).toEqual({
      'a.tsx': 'b.tsx',
    })
    using output = engine.codeExtract(
      'b.tsx',
      box('insetX={1}'),
      '@devup-ui/react',
      './css',
      false,
      false,
      true,
      {},
    )
    expect(output.code).toContain('du-')
    expect(engine.getCss(undefined, false)).toContain('left:')
  })

  it('overwrites what an earlier configuration left with the defaults', () => {
    const app = context()
    const engine = createWasm(app.root)
    engine.setDebug(true)
    engine.setPrefix('old-')
    engine.registerShorthands({ insetX: ['left', 'right'] })
    engine.registerTheme({ colors: { default: { primary: 'red' } } })
    engine.importCanonicalMap({ 'a.tsx': 'b.tsx' })
    engine.importFileRoutes({ 'b.tsx': [0, 1] })
    engine.setAtomHoist(2)

    createEngineConfigurer(app, { theme: {}, plan })(engine)

    expect(engine.isDebug()).toBe(false)
    expect(engine.getPrefix()).toBeUndefined()
    expect(engine.getCss(undefined, false)).not.toContain('--primary')
    expect(JSON.parse(engine.exportCanonicalMap())).toEqual({})
    using output = engine.codeExtract(
      'b.tsx',
      box('insetX={1}'),
      '@devup-ui/react',
      './css',
      false,
      false,
      true,
      {},
    )
    expect(output.code).not.toContain('old-')
    expect(engine.getCss(0, false)).not.toContain('left:')
  })

  it('resolves imports against the captured root after the working directory changes', () => {
    const app = context()
    writeFileSync(join(app.root, 'color.ts'), 'export const color = "purple"')
    const engine = createWasm(app.root)
    createEngineConfigurer(app, { theme: {}, plan })(engine)
    process.chdir(makeProject())

    using output = engine.codeExtract(
      'page.tsx',
      `import { Box } from '@devup-ui/react'\nimport { color } from './color'\nexport const C = () => <Box bg={color} />`,
      '@devup-ui/react',
      './css',
      false,
      false,
      true,
      {},
    )

    expect(output.dependencies).toEqual(['color.ts'])
    expect(engine.getCss(0, false)).toContain('background:purple')
  })
})
