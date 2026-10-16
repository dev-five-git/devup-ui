import * as fs from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import { createDirectoryExclusion } from '../directory-exclusion'
import { createProductionPackageRoots } from '../production-package-roots'
import {
  createResolutionInputs,
  type ResolutionInputs,
} from '../resolution-inputs'

async function fixture(
  run: (root: string, file: (path: string) => string) => void,
) {
  const root = fs.realpathSync.native(
    fs.mkdtempSync(join(tmpdir(), 'devup-p2-roots-')),
  )
  const file = (path: string) => {
    const target = join(root, path)
    fs.mkdirSync(dirname(target), { recursive: true })
    fs.writeFileSync(target, '{}')
    return target
  }
  try {
    run(root, file)
  } finally {
    fs.rmSync(root, { recursive: true, force: true })
  }
}

it.each([false, true])(
  'retains local cohorts and cumulative evidence with observer=%s',
  async (observe) => {
    await fixture((root, file) => {
      // Given distinct starts and unrelated caller-owned history.
      const manifest = file('node_modules/chosen/package.json')
      const starts = ['left', 'right'].map((side) =>
        dirname(file(`${side}/marker`)),
      )
      const sentinel = join(root, 'sentinel')
      const inputs = createResolutionInputs({
        fileDependencies: [sentinel],
        missingDependencies: [sentinel],
      })
      const observed: ResolutionInputs[] = []
      const roots = createProductionPackageRoots(
        ['chosen'],
        createDirectoryExclusion(),
        {
          inputs,
          observer: observe
            ? (snapshot) => {
                expect(inputs.snapshot().fileDependencies).toEqual(
                  expect.arrayContaining(snapshot.fileDependencies),
                )
                expect(inputs.snapshot().missingDependencies).toEqual(
                  expect.arrayContaining(snapshot.missingDependencies),
                )
                observed.push(snapshot)
              }
            : undefined,
        },
      )
      const expected = new Set<string>()
      // When independent lexical starts discover the same real package, then reuse their caches.
      for (const start of starts) {
        const offset = observed.length
        let ancestors = 0
        for (let directory = start; ; directory = dirname(directory)) {
          ancestors++
          for (const scope of ['@devup-ui', '@devup-editor'])
            expected.add(join(directory, 'node_modules', scope))
          const candidate = join(directory, 'node_modules/chosen/package.json')
          if (candidate !== manifest) expected.add(candidate)
          if (dirname(directory) === directory) break
        }
        const result = roots(start)
        expect(result).toEqual([dirname(manifest)])
        expect(observed.length - offset).toBe(observe ? 2 * ancestors + 1 : 0)
        const read = spyOn(fs, 'readdirSync')
        try {
          expect(roots(start)).toBe(result)
          expect(read).not.toHaveBeenCalled()
          expect(observed.length - offset).toBe(observe ? 2 * ancestors + 1 : 0)
        } finally {
          read.mockRestore()
        }
        for (const snapshot of observed.slice(offset)) {
          expect(snapshot.fileDependencies).not.toContain(sentinel)
          expect(snapshot.missingDependencies).not.toContain(sentinel)
          for (const other of starts.filter((path) => path !== start))
            expect(snapshot.missingDependencies).not.toContain(
              join(other, 'node_modules/chosen/package.json'),
            )
        }
      }
      // Then the watch union and original collector retain every consulted lexical input.
      expect(inputs.snapshot()).toEqual({
        fileDependencies: [manifest, sentinel].sort(),
        missingDependencies: [sentinel, ...expected].sort(),
      })
      if (observe) {
        expect([
          ...new Set(observed.flatMap((snapshot) => snapshot.fileDependencies)),
        ]).toEqual([manifest])
        expect(
          [
            ...new Set(
              observed.flatMap((snapshot) => snapshot.missingDependencies),
            ),
          ].sort(),
        ).toEqual([...expected].sort())
      }
      const fatal = join(
        dirname(file('fatal/marker')),
        'node_modules/chosen/package.json',
      )
      const cause = Object.assign(new Error('denied'), { code: 'EACCES' })
      const original = fs.statSync
      const stat = spyOn(fs, 'statSync').mockImplementation(
        new Proxy(original, {
          apply(target, receiver, args) {
            if (args[0] === fatal) throw cause
            return Reflect.apply(target, receiver, args)
          },
        }),
      )
      try {
        expect(() => roots(join(root, 'fatal'))).toThrow(cause)
        expect(inputs.snapshot().fileDependencies).toContain(fatal)
        expect(
          observed.flatMap((snapshot) => snapshot.fileDependencies),
        ).not.toContain(fatal)
      } finally {
        stat.mockRestore()
      }
    })
  },
)
