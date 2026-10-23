import { mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { dirname } from 'node:path'

import { createNonphysicalIdStore } from '@devup-ui/plugin-utils'
import { expect, it, spyOn } from 'bun:test'
import type { Plugin } from 'vite'

import { DevupUI } from '../plugin'
import { ProductionGeneration } from '../production-engine'
import { productionFixture } from './production-fixture'

it('propagates native rewrite failure when the numbering path is obstructed after preparation', async () => {
  // Given an ordinary prepared client and a genuine opaque styling source.
  const fixture = await productionFixture()
  const opaque = '\0native:rewrite-style.js'
  const { filePath } = createNonphysicalIdStore({
    integration: 'Vite',
    resolvedRoot: fixture.root,
    contextKey: 'client',
    distDir: `${fixture.root}/df`,
  })
  let expectedCode: string | undefined
  let extracted: string | undefined
  let rendered = false
  const events: string[] = []
  const incoming: (Error | undefined)[] = []
  const finishing: ProductionGeneration[] = []
  const failed: ProductionGeneration[] = []
  const restores: (() => void)[] = []
  let rewriteFailure: Error | undefined
  let finishFailure: Error | undefined
  const provider: Plugin = {
    name: 'native-rewrite-obstruction',
    enforce: 'post',
    resolveId(id) {
      if (id === 'virtual-style') return opaque
    },
    async load(id) {
      if (id !== opaque) return
      await readFile(`${fixture.root}/df/theme.d.ts`, 'utf8')
      await rm(dirname(filePath), { recursive: true, force: true })
      await writeFile(dirname(filePath), 'not a directory')
      try {
        await mkdir(dirname(filePath), { recursive: true })
      } catch (error) {
        if (!(error instanceof Error) || !('code' in error)) throw error
        if (typeof error.code !== 'string') throw error
        expectedCode = error.code
      }
      if (expectedCode === undefined)
        throw new TypeError(
          'Expected independent numbering-store mkdir failure',
        )
      return "import {css} from '@devup-ui/react'; export const cls=css({color:'red'});"
    },
    transform(code, id) {
      if (id === opaque) extracted = code
    },
    renderStart() {
      rendered = true
    },
  }
  try {
    await fixture.file('app/src/main.js', "export {cls} from 'virtual-style';")
    const finish = ProductionGeneration.prototype.finish
    const fail = ProductionGeneration.prototype.fail
    const finishSpy = spyOn(
      ProductionGeneration.prototype,
      'finish',
    ).mockImplementation(async function (this: ProductionGeneration, ...args) {
      finishing.push(this)
      incoming.push(args[2])
      events.push('finish')
      const store = this.runtime(args[0]).plan.store
      const rewrite = store.rewrite
      const rewriteSpy = spyOn(store, 'rewrite').mockImplementation(
        async (...ids) => {
          try {
            return await rewrite.apply(store, ids)
          } catch (error) {
            if (!(error instanceof Error)) throw error
            rewriteFailure = error
            events.push('rewrite-rejected')
            throw error
          }
        },
      )
      restores.push(() => rewriteSpy.mockRestore())
      try {
        return await finish.apply(this, args)
      } catch (error) {
        if (!(error instanceof Error)) throw error
        finishFailure = error
        events.push('finish-rejected')
        throw error
      }
    })
    restores.push(() => finishSpy.mockRestore())
    const failSpy = spyOn(
      ProductionGeneration.prototype,
      'fail',
    ).mockImplementation(function (this: ProductionGeneration) {
      failed.push(this)
      events.push('fail')
      return fail.call(this)
    })
    restores.push(() => failSpy.mockRestore())
    // When native input completes and its normal buildEnd rewrites the store.
    let failure: Error | undefined
    try {
      await fixture.run(false, [], {
        plugins: [DevupUI({ singleCss: false }), provider],
      })
    } catch (error) {
      if (!(error instanceof Error)) throw error
      failure = error
    }
    // Then real extraction precedes the propagated OS fault, never output rendering.
    expect(extracted).toMatch(/\bcls\s*=\s*['"][^'"]+['"]/)
    expect(extracted).not.toContain('@devup-ui/react')
    expect(expectedCode).toBeDefined()
    expect(failure).toBeInstanceOf(Error)
    expect(failure?.message).toContain(`${expectedCode}:`)
    expect(failure?.message).toContain('mkdir')
    expect(rendered).toBe(false)
    expect(incoming).toEqual([undefined])
    expect(finishing).toHaveLength(1)
    expect(failed).toHaveLength(1)
    expect(failed[0]).toBe(finishing[0])
    expect(rewriteFailure).toBeInstanceOf(Error)
    expect(finishFailure).toBe(rewriteFailure)
    expect(events).toEqual([
      'finish',
      'rewrite-rejected',
      'fail',
      'finish-rejected',
    ])
  } finally {
    for (const restore of restores.reverse()) restore()
    await fixture.close()
  }
})
