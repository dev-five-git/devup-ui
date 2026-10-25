import * as fs from 'node:fs'
import { dirname, join } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import * as scanner from '../import-scanner'
import { PreparationInputs } from '../preparation-inputs'
import { readResolutionFile } from '../resolution-inputs'
import { createPreparedFixture } from './prepared-graph-fixture'

let root: string
const file = createPreparedFixture((path) => {
  root = path
})

it('shares empty successful reads but retains independent lexical symlink inputs', () => {
  // Given two lexical links to one empty physical source.
  const physical = file('physical/empty.js', '')
  fs.mkdirSync(join(root, 'links'))
  for (const name of ['a', 'b'])
    fs.symlinkSync(dirname(physical), join(root, 'links', name), 'junction')
  const paths = ['a', 'b'].map((name) => join(root, 'links', name, 'empty.js'))
  const epoch = new PreparationInputs()
  const original = fs.readFileSync
  const read = spyOn(fs, 'readFileSync')
  try {
    // When independent consumers read each lexical input twice.
    const snapshots = paths.flatMap((path) =>
      [0, 1].map(() => {
        const inputs = epoch.collect()
        expect(readResolutionFile(path, inputs)).toBe('')
        return inputs.snapshot()
      }),
    )
    // Then empty text hits while each consumer records its own lexical dependency.
    expect(read).toHaveBeenCalledTimes(2)
    expect(snapshots).toEqual(
      paths.flatMap((path) =>
        [0, 1].map(() => ({
          fileDependencies: [path],
          missingDependencies: [],
        })),
      ),
    )
    fs.writeFileSync(physical, 'changed')
    expect(original(paths[0], 'utf8')).toBe('changed')
    expect(epoch.read(paths[0])).toBe('')
    epoch.dispose()
    expect(new PreparationInputs().read(paths[0])).toBe('changed')
  } finally {
    read.mockRestore()
  }
})

it.each(['ENOENT', 'EACCES'])(
  'retries failed reads and preserves exact %s repair observations',
  (code) => {
    // Given a narrow IO fault; all other IO still calls the real filesystem.
    const path = file('blocked.js')
    const cause = Object.assign(new Error('fault'), { code })
    const original = fs.readFileSync
    let attempts = 0
    const read = spyOn(fs, 'readFileSync').mockImplementation(
      new Proxy(original, {
        apply(target, receiver, args) {
          if (args[0] === path && attempts++ < 2) throw cause
          return Reflect.apply(target, receiver, args)
        },
      }),
    )
    const epoch = new PreparationInputs()
    try {
      // When separate consumers encounter the same failed input before its repair.
      for (let index = 0; index < 2; index++) {
        const inputs = epoch.collect()
        let caught: unknown
        try {
          readResolutionFile(path, inputs)
        } catch (error) {
          caught = error
        }
        // Then neither the failure object nor the repair observation is memoized away.
        expect(caught).toBe(cause)
        expect(inputs.snapshot()).toEqual({
          fileDependencies: code === 'EACCES' ? [path] : [],
          missingDependencies: code === 'ENOENT' ? [path] : [],
        })
      }
      expect(epoch.read(path)).toBe('export {}')
      expect(attempts).toBe(3)
    } finally {
      read.mockRestore()
    }
  },
)

it('executes the real scanner once per effective bytes and grammar tuple', () => {
  // Given a scanner call-through and one preparation epoch.
  const epoch = new PreparationInputs()
  const scan = spyOn(scanner, 'scanImportRequests')
  const code = "import './a.js'; const view=<div>{import('./b.js')}</div>;"
  try {
    // When duplicate bytes are scanned alongside distinct source and grammar.
    for (const [source, jsx, ts] of [
      [code, true, true],
      [code, false, true],
      [code, true, false],
      ["import './c.js'", true, true],
    ] as const) {
      for (let repeat = 0; repeat < 2; repeat++) {
        const references = epoch.scan(source, jsx, ts)
        expect(references.map(({ specifier }) => specifier)).toEqual(
          source === code ? ['./a.js', './b.js'] : ['./c.js'],
        )
      }
    }
    // Then only actual scanner executions, not fabricated graph edges, are counted.
    expect(scan).toHaveBeenCalledTimes(4)
  } finally {
    scan.mockRestore()
  }
})
