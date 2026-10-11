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

it('discovers both implicit scopes and exact includes with lexical-start-local caching', async () => {
  await fixture((root, file) => {
    // Given an ancestor installation, a nested copy and non-admitted neighbors.
    const parent = ['@devup-ui/z', '@devup-editor/editor', 'chosen'].map(
      (name) => dirname(file(`node_modules/${name}/package.json`)),
    )
    const nested = dirname(
      file('outside/node_modules/@devup-ui/z/package.json'),
    )
    file('node_modules/chosen-other/package.json')
    const observed: ResolutionInputs[] = []
    const roots = createProductionPackageRoots(
      ['chosen', 'missing'],
      createDirectoryExclusion(),
      {
        inputs: createResolutionInputs(),
        observer: (inputs) => observed.push(inputs),
      },
    )
    // When the same start is reused and a different lexical start is independently discovered.
    const first = roots(root)
    const read = spyOn(fs, 'readdirSync')
    try {
      expect(roots(root)).toBe(first)
      expect(read).not.toHaveBeenCalled()
      expect(roots(join(root, 'outside'))).toEqual([...parent, nested].sort())
    } finally {
      read.mockRestore()
    }
    // Then selection is exact, and lexical consulted manifests/misses remain observable.
    expect(first).toEqual(parent.sort())
    expect(
      observed.flatMap(({ fileDependencies }) => fileDependencies),
    ).toContain(join(nested, 'package.json'))
    expect(
      observed.flatMap(({ missingDependencies }) => missingDependencies),
    ).toContain(join(root, 'node_modules/missing/package.json'))
  })
})

it.each(['scope', 'package'] as const)(
  'excludes lexical and physical %s paths before inventory',
  async (kind) => {
    await fixture((root, file) => {
      // Given aliases into blocked physical output and a lexical blocked sibling.
      const physical = dirname(file('blocked/package.json'))
      const scope = join(root, 'node_modules/@devup-ui')
      fs.mkdirSync(dirname(scope), { recursive: true })
      if (kind === 'scope') fs.symlinkSync(physical, scope, 'junction')
      else {
        fs.mkdirSync(scope)
        fs.symlinkSync(physical, join(scope, 'linked'), 'junction')
      }
      const lexical = dirname(
        file('node_modules/@devup-editor/blocked/package.json'),
      )
      const excluded = createDirectoryExclusion([
        physical,
        lexical,
        join(root, 'node_modules/@devup-editor'),
      ])
      const inputs = createResolutionInputs()
      const roots = createProductionPackageRoots([], excluded, {
        inputs,
        observer: undefined,
      })
      // When direct discovery respects physical and lexical exclusion independently.
      const result = roots(root)
      // Then neither alias nor lexical scope can become a distribution origin.
      expect(result).toEqual([])
    })
  },
)

it.each(['ENOTDIR', 'EACCES', 'non-error'])(
  'retains direct discovery outcome for %s scope faults',
  async (code) => {
    await fixture((root, file) => {
      // Given a real existing scope and a narrow fault at its directory read boundary.
      file('node_modules/@devup-ui/pkg/package.json')
      const scope = join(root, 'node_modules/@devup-ui')
      const cause =
        code === 'non-error'
          ? 'fault'
          : Object.assign(new Error(code), { code })
      const original = fs.readdirSync
      const read = spyOn(fs, 'readdirSync').mockImplementation(
        new Proxy(original, {
          apply(target, receiver, args) {
            if (args[0] === scope) throw cause
            return Reflect.apply(target, receiver, args)
          },
        }),
      )
      const inputs = createResolutionInputs()
      const observed: ResolutionInputs[] = []
      const roots = createProductionPackageRoots(
        [],
        createDirectoryExclusion(),
        {
          inputs,
          observer: (snapshot) => observed.push(snapshot),
        },
      )
      try {
        // When direct discovery sees a missing-directory class or a fatal original cause.
        if (code === 'ENOTDIR') expect(roots(root)).toEqual([])
        else {
          let caught: unknown
          try {
            roots(root)
          } catch (failure) {
            caught = failure
          }
          expect(caught).toBe(cause)
        }
        // Then the finally observer preserves the actual lexical repair input.
        expect(
          observed.flatMap((snapshot) =>
            code === 'ENOTDIR'
              ? snapshot.missingDependencies
              : snapshot.fileDependencies,
          ),
        ).toContain(scope)
      } finally {
        read.mockRestore()
      }
    })
  },
)
