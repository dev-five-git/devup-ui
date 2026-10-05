import { join } from 'node:path'

import { afterEach, expect, it, mock, spyOn } from 'bun:test'

import { resetInit, setWasmForTesting } from '../loader'
import { createWasm, withModuleResolver } from '../wasm'
import { invoke } from './loader-fixture'
import { box, installProjectHooks, makeProject } from './project'

installProjectHooks()
afterEach(() => {
  mock.restore()
  setWasmForTesting(undefined)
})

it.each([false, true])(
  'releases real legacy-loader outputs even when map parsing fails=%s',
  async (invalidMap) => {
    // Given
    const root = makeProject()
    const engine = createWasm(root)
    const source = box('bg="red"')
    const initialSheet = JSON.parse(engine.exportSheet())
    const nativeExtract = engine.codeExtract
    let released = 0
    spyOn(engine, 'codeExtract').mockImplementation((...args) => {
      const output = nativeExtract(...args)
      const free = output.free.bind(output)
      spyOn(output, 'free').mockImplementation(() => {
        released += 1
        free()
      })
      if (invalidMap)
        Object.defineProperty(output, 'map', { value: 'invalid JSON' })
      return output
    })
    setWasmForTesting(engine)
    resetInit()
    // When
    const run = invoke(
      {
        projectRoot: root,
        package: '@devup-ui/react',
        cssDir: join(root, 'styles'),
        defaultSheet: initialSheet,
        theme: {},
      },
      join(root, 'page.tsx'),
      source,
    )
    // Then
    if (invalidMap) await expect(run.result).rejects.toBeInstanceOf(SyntaxError)
    else expect((await run.result).code).not.toContain('<Box')
    expect(released).toBe(1)
  },
)

it('remaps prepared errors from the direct legacy-loader fallback', async () => {
  // Given
  const root = makeProject({ 'page.mdx': '# raw Markdown' })
  const filename = join(root, 'page.mdx')
  const engine = createWasm(root)
  const source = `import { css } from '@devup-ui/react'\nconst v = Math.random()\nexport const c = css({ bg: v })`
  withModuleResolver(engine, root, {
    prepareSource: () => ({
      code: source,
      map: { version: 3, sources: [filename], mappings: ';;AAKA', names: [] },
    }),
  })
  setWasmForTesting(engine)
  resetInit()
  // When
  const run = invoke(
    {
      projectRoot: root,
      package: '@devup-ui/react',
      defaultSheet: JSON.parse(engine.exportSheet()),
      theme: {},
    },
    filename,
    source,
  )
  // Then
  await expect(run.result).rejects.toThrow(`${filename}:6:1:`)
})
