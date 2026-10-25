import * as fs from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { PreparationInputs as ProductionInputs } from '@devup-ui/plugin-utils'
import { afterEach, beforeEach, expect, it, spyOn } from 'bun:test'

import type { ResolutionInputs } from '../../../plugin-utils/src/resolution-inputs'
import { productionRootClosure } from '../production-root-closure'

let root: string
beforeEach(() => {
  root = fs.realpathSync.native(
    fs.mkdtempSync(join(tmpdir(), 'devup-vite-root-inputs-')),
  )
})
afterEach(() => {
  fs.rmSync(root, { recursive: true, force: true })
})

it.each(['ENOENT', 'EACCES'])(
  'keeps root wrapper, cause and finally observations fresh after %s',
  (code) => {
    // Given a real root whose read fails at the filesystem boundary.
    const path = join(root, 'missing.js')
    const cause = Object.assign(new Error('root read fault'), { code })
    const original = fs.readFileSync
    let attempts = 0
    const read = spyOn(fs, 'readFileSync').mockImplementation(
      new Proxy(original, {
        apply(target, receiver, args) {
          if (args[0] === path) {
            attempts++
            throw cause
          }
          return Reflect.apply(target, receiver, args)
        },
      }),
    )
    const observations: ResolutionInputs[] = []
    const context = {
      key: 'root',
      files: [{ path, realPath: path }],
      toId: (path: string) => path,
      resolverOptions: {
        cwd: root,
        onResolutionInputs: (inputs: ResolutionInputs) =>
          observations.push(inputs),
      },
    }
    const epoch = new ProductionInputs()
    try {
      // When independent root consumers retry the same failed read in one epoch.
      for (let repeat = 0; repeat < 2; repeat++) {
        let caught: unknown
        try {
          productionRootClosure(
            context,
            {},
            { selected: () => true, relevant: () => true },
            epoch,
          )
        } catch (error) {
          caught = error
        }
        // Then the wrapper keeps the exact cause and its established file-only finally receipt.
        expect(caught).toMatchObject({ name: 'RootSourceError', cause })
        if (!(caught instanceof Error))
          throw new TypeError('Expected root failure')
        expect(caught.cause).toBe(cause)
        expect(observations.at(-1)).toEqual({
          fileDependencies: [path],
          missingDependencies: [],
        })
      }
      expect(attempts).toBe(2)
    } finally {
      read.mockRestore()
    }
  },
)
