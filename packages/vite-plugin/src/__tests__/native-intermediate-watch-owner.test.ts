import * as fs from 'node:fs/promises'

import { expect, it, spyOn } from 'bun:test'
import * as vite from 'vite'

import { productionFixture } from './production-fixture'

it('waits through an intermediate native watch completion before returning updated source', async () => {
  // Given the real watch fixture and a filesystem edit with an intermediate revision.
  const fixture = await productionFixture()
  const build = vite.build
  const write = fs.writeFile
  const finalWrite = Promise.withResolvers<void>()
  let afterIntermediate: ((action: () => void) => void) | undefined
  using _build = spyOn(vite, 'build').mockImplementation(async (...args) => {
    const result = await build(...args)
    if (!Array.isArray(result) && 'on' in result) {
      afterIntermediate = (action) => {
        const listener = (event: { readonly code: string }) => {
          if (event.code !== 'BUNDLE_END') return
          result.off('event', listener)
          action()
        }
        result.on('event', listener)
      }
    }
    return result
  })
  using _write = spyOn(fs, 'writeFile').mockImplementation(
    async (path, data, options) => {
      if (
        typeof path === 'string' &&
        path.replaceAll('\\', '/') === `${fixture.root}/src/unrelated.js` &&
        data === 'export const value=2;'
      ) {
        if (afterIntermediate === undefined)
          throw new TypeError('Expected native watcher before editing source')
        afterIntermediate(() => {
          void write(path, data, options).then(
            finalWrite.resolve,
            finalWrite.reject,
          )
        })
        return write(
          path,
          'export const value=1; // intermediate revision',
          options,
        )
      }
      return write(path, data, options)
    },
  )
  try {
    // When actual native watch completes an intermediate revision before the final edit.
    const { first, second, ids } = await fixture.watchGeneration(false)
    await finalWrite.promise
    // Then the helper returns the requested final source while retaining styling ownership.
    expect(first?.code).toMatch(/value\s*=\s*1/)
    expect(second?.code).toMatch(/value\s*=\s*2/)
    expect(second?.css).toBe(first?.css)
    expect(ids).toContain('\0native:watch-style.js')
  } finally {
    await fixture.close()
  }
}, 30000)

it('rejects a native watcher where the completed-output fixture requires a bundle', async () => {
  // Given a real native watch request passed to the completed-output helper.
  const fixture = await productionFixture()
  const entry = await fixture.file('app/src/main.js', 'export const value=42')
  const build = vite.build
  let close: (() => Promise<void>) | undefined
  using _build = spyOn(vite, 'build').mockImplementation(async (...args) => {
    const result = await build(...args)
    if (!Array.isArray(result) && 'on' in result) close = () => result.close()
    return result
  })
  try {
    // When the real builder returns its native watcher rather than bundle output.
    const output = fixture.run(false, [], {
      build: { watch: {}, write: false, lib: { entry, formats: ['es'] } },
    })
    // Then the helper rejects the wrong return shape without crediting output.
    await expect(output).rejects.toThrow(TypeError)
  } finally {
    await close?.()
    await fixture.close()
  }
}, 30000)
